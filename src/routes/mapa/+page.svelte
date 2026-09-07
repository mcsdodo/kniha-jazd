<script lang="ts">
	import 'leaflet/dist/leaflet.css';
	import { onMount, onDestroy } from 'svelte';
	import { page } from '$app/stores';
	import type { Map as LeafletMap, Polyline, Marker, LeafletMouseEvent } from 'leaflet';
	import {
		generateRoute,
		getTripRoute,
		saveTripRoute,
		deleteTripRoute,
		getTrips,
		startRouteForTrip,
		routeDirect,
		savePlace
	} from '$lib/api';
	import type {
		GeneratedRoute,
		RouteMap,
		Trip,
		RouteMode,
		Waypoint,
		InsertPoint,
		Place,
		PlaceSource
	} from '$lib/types';
	import { activeVehicleStore } from '$lib/stores/vehicles';
	import { toast } from '$lib/stores/toast';
	import ConfirmModal from '$lib/components/ConfirmModal.svelte';
	import PlaceModal from '$lib/components/PlaceModal.svelte';
	import LL from '$lib/i18n/i18n-svelte';

	/** Just enough to route from an endpoint — not the full place-book `Place`. */
	type Endpoint = { lat: number; lon: number; displayName: string };

	// Map view before the first route arrives. Every render calls fitBounds, so
	// this is only ever visible for the moment between mount and first draw.
	const INITIAL_CENTER: [number, number] = [48.7, 19.7];
	const INITIAL_ZOOM = 7;

	let tripId = $derived($page.url.searchParams.get('trip') ?? '');

	let trip = $state<Trip | null>(null);
	/** Route persisted against the trip. Only save/remove ever change this. */
	let savedRoute = $state<RouteMap | null>(null);
	/** Freshly generated, not persisted. Regenerating writes nothing. */
	let generated = $state<GeneratedRoute | null>(null);
	/** Decided by the backend (`mode_for`) — this page never compares origin
	 *  to destination itself. Null until the first plan or saved route loads. */
	let mode = $state<RouteMode | null>(null);
	/** Alternatives for the current direct route, in the backend's order. */
	let alternatives = $state<GeneratedRoute[]>([]);
	let activeIndex = $state(0);
	/** Endpoints as resolved so far, for building waypoint lists. Populated
	 *  only from a fresh plan (`startForTrip`) or a saved route's own
	 *  waypoints (`rehydrateEndpoints`) — never from the place dialog, so
	 *  there is exactly one writer for each. */
	let resolvedOrigin = $state<Endpoint | null>(null);
	let resolvedDestination = $state<Endpoint | null>(null);
	/** Which endpoint has no coordinate in the place book yet, if any. Drives
	 *  the shared place dialog (PlaceModal, task 75) opened in place. */
	let unplacedField = $state<'origin' | 'destination' | null>(null);

	let loading = $state(true);
	let generating = $state(false);
	let saving = $state(false);
	let removing = $state(false);
	let error = $state<string | null>(null);
	/** False for errors a retry cannot fix, so the button is not offered. */
	let retryable = $state(true);
	let confirmingRemove = $state(false);
	let savedNotice = $state(false);

	let mapEl = $state<HTMLDivElement | null>(null);
	let leafletReady = $state(false);
	let mapReady = $state(false);

	// Plain (non-reactive) handles: Leaflet objects are mutable and must never
	// become effect dependencies.
	let leaflet: typeof import('leaflet') | null = null;
	let map: LeafletMap | null = null;
	let routeLayer: Polyline | null = null;
	let inactiveLayers: Polyline[] = [];
	let waypointMarkers: Marker[] = [];
	let ghost: Marker | null = null;
	/** True between the ghost's mousedown and its dragend/mouseup, so a
	 *  `mouseout` on the line underneath -- which fires both mid-drag and as
	 *  a same-tick side effect of the ghost's own creation, see `attachGhost`
	 *  -- cannot destroy the handle before or during a real drag. */
	let dragging = false;
	/** Set by selectAlternative, consumed once by the draw effect, so picking
	 *  an alternative redraws the lines without re-zooming the map -- fitting
	 *  bounds still happens on every other draw (first load, regenerate).
	 *  Dragging a handle or the ghost sets it too, for the same reason: an
	 *  edit reroutes to a concrete road result and the map should not jump. */
	let skipFit = false;
	let dataLoadStarted = false;

	let displayRoute = $derived<GeneratedRoute | RouteMap | null>(generated ?? savedRoute);
	/** Reads `displayRoute`, not just `generated`, so a re-opened saved direct
	 *  route with vias also reaches the "alternatives unavailable" branch below
	 *  -- without this, that branch was only reachable right after a fresh
	 *  `runDirect` proposal, never on cold load (task 72's I2 fix only covered
	 *  the freshly-generated case). Safe for loop mode too: both branches that
	 *  read this are gated on `mode === 'direct'`, so a saved loop route's own
	 *  multi-stop waypoint list never lights them up. */
	let hasVias = $derived((displayRoute?.waypoints.length ?? 0) > 2);
	let stopNames = $derived(
		displayRoute
			? displayRoute.waypoints.map((w) => w.name).filter((name): name is string => !!name)
			: []
	);
	// Both come from the backend. The tolerance is a business rule and has one
	// home in Rust (ADR-008); deriving it here would measure road distance
	// against a threshold the algorithm applies to a different quantity, so the
	// page could flag a route the backend considers perfectly in tolerance.
	let deviationPercent = $derived(displayRoute?.deviationPercent ?? null);
	let deviationOffTarget = $derived(displayRoute?.offTarget ?? false);
	let busy = $derived(loading || generating || saving || removing);

	onMount(async () => {
		// Leaflet touches `window` at import time — keep it out of the module graph.
		leaflet = (await import('leaflet')).default;
		leafletReady = true;
	});

	onDestroy(() => {
		routeLayer = null;
		waypointMarkers = [];
		ghost = null;
		map?.remove();
		map = null;
	});

	// Create the map once Leaflet and the container element exist. Not done in
	// onMount: Leaflet is loaded lazily, so the container element and the library
	// only both exist some time after mount.
	$effect(() => {
		if (!leafletReady || !mapEl || map) return;
		const L = leaflet;
		if (!L) return;

		map = L.map(mapEl).setView(INITIAL_CENTER, INITIAL_ZOOM);
		L.tileLayer('https://tile.openstreetmap.org/{z}/{x}/{y}.png', {
			maxZoom: 19,
			// Required by the OpenStreetMap tile usage policy.
			attribution:
				'&copy; <a href="https://www.openstreetmap.org/copyright">OpenStreetMap</a> contributors'
		}).addTo(map);
		mapReady = true;
	});

	// Draw whatever route is currently on display, plus any inactive
	// alternatives underneath it.
	$effect(() => {
		const route = displayRoute;
		const alts = alternatives;
		const active = activeIndex;
		if (!mapReady || !map || !leaflet) return;

		// Consumed exactly once per draw, regardless of which branch below
		// returns early, so a stale `true` can never leak into a later,
		// unrelated redraw.
		const shouldSkipFit = skipFit;
		skipFit = false;

		if (routeLayer) {
			map.removeLayer(routeLayer);
			routeLayer = null;
		}
		for (const layer of inactiveLayers) {
			map.removeLayer(layer);
		}
		inactiveLayers = [];

		// Inactive alternatives sit UNDER the active line and are clickable.
		alts.forEach((route, i) => {
			if (i === active || route.coordinates.length === 0) return;
			const layer = leaflet!
				.polyline(route.coordinates, { color: '#94a3b8', weight: 4, opacity: 0.6 })
				.addTo(map!);
			layer.on('click', () => selectAlternative(i));
			inactiveLayers.push(layer);
		});

		if (route && route.coordinates.length > 0) {
			routeLayer = leaflet
				.polyline(route.coordinates, { color: '#0066cc', weight: 5, opacity: 0.85 })
				.addTo(map);
			attachGhost(routeLayer);
			if (!shouldSkipFit) {
				map.fitBounds(routeLayer.getBounds(), { padding: [30, 30] });
			}
		}

		// Runs on every draw, including the empty-route branch above, so
		// handles left over from a route that just got removed (or a load
		// that just failed) do not linger on the map with no line under them.
		drawHandles();
	});

	/** Small circular handle. Endpoints are visually heavier than vias. */
	function handleIcon(L: typeof import('leaflet'), endpoint: boolean) {
		return L.divIcon({
			className: endpoint ? 'wp-handle wp-endpoint' : 'wp-handle',
			iconSize: [endpoint ? 14 : 10, endpoint ? 14 : 10]
		});
	}

	/**
	 * Draggable handles for every waypoint of the route on display, endpoints
	 * included. Dragging one re-routes through its new position on release;
	 * clicking a via (not an endpoint) removes it and re-routes without it.
	 */
	function drawHandles() {
		if (!map || !leaflet) return;
		waypointMarkers.forEach((m) => map!.removeLayer(m));
		waypointMarkers = [];

		const points = currentWaypoints();
		points.forEach((wp, i) => {
			const endpoint = i === 0 || i === points.length - 1;
			const marker = leaflet!
				.marker([wp.lat, wp.lon], {
					draggable: true,
					icon: handleIcon(leaflet!, endpoint)
				})
				.addTo(map!);

			// ONE request, on release. Never during the drag: the routing
			// service is capped at a request a second, and mid-drag routing
			// would spend that budget on frames nobody sees.
			marker.on('dragend', () => {
				const { lat, lng } = marker.getLatLng();
				const next = points.map((p, j) => (j === i ? { ...p, lat, lon: lng } : p));
				void reroute(next);
			});

			// Clicking a via removes it. Endpoints are not removable — that
			// would change where the journey started or ended.
			if (!endpoint) {
				marker.bindTooltip($LL.routeMap.removeWaypoint());
				marker.on('click', () => {
					void reroute(points.filter((_, j) => j !== i));
				});
			}

			waypointMarkers.push(marker);
		});
	}

	/**
	 * Ghost handle: appears on the active line under the cursor, and dragging
	 * it off creates a new waypoint. Where that waypoint LANDS in the ordered
	 * list is decided by the backend, using the same placement geometry as
	 * generation (route_maps.rs::insert_waypoint) -- never recomputed here.
	 */
	function attachGhost(layer: Polyline) {
		if (!map || !leaflet) return;
		layer.on('mousemove', (e: LeafletMouseEvent) => {
			if (!ghost) {
				ghost = leaflet!
					.marker(e.latlng, { draggable: true, icon: handleIcon(leaflet!, false) })
					.addTo(map!);
				// Armed on `mousedown`, ahead of Leaflet's own `dragstart`:
				// `dragstart` only fires once movement is detected, and the
				// very next native event after the ghost appears -- often the
				// `mousedown` itself -- already carries a `mouseout` for the
				// line underneath it (see the comment on the `mouseout`
				// handler below). Arming this early costs nothing and closes
				// that window completely.
				ghost.on('mousedown', () => {
					dragging = true;
				});
				// A plain click (mousedown+mouseup with no real movement)
				// never fires `dragend`, so reset the guard here too --
				// otherwise a click-without-drag would leave `dragging` stuck
				// true and the next hover-away could never clean up its ghost.
				ghost.on('mouseup', () => {
					dragging = false;
				});
				ghost.on('dragend', () => {
					dragging = false;
					const { lat, lng } = ghost!.getLatLng();
					const polyline = generated?.polyline ?? savedRoute?.polyline ?? '';
					map!.removeLayer(ghost!);
					ghost = null;
					void reroute(currentWaypoints(), { lat, lon: lng, polyline });
				});
			} else {
				ghost.setLatLng(e.latlng);
			}
		});

		// Without this the ghost outlives the hover: move the cursor off the
		// line and a stray draggable dot stays behind, and because creation is
		// guarded by `if (!ghost)`, hovering elsewhere reuses that stale one
		// rather than placing a fresh handle under the cursor.
		//
		// A `mouseout` on the line also fires as a side effect that has
		// nothing to do with the cursor actually leaving it: inserting the
		// ghost marker on top of the line, exactly under the cursor, does not
		// itself fire a transition, but the browser's hover tracking is still
		// pointing at the line (that is what the `mousemove` which created
		// the ghost was resolved against, before the handler above added
		// anything on top of it) -- so it owes a catch-up "line to ghost"
		// mouseout/mouseover pair, and fires it ahead of whatever native
		// event comes next, even `mousedown` with the pointer never having
		// moved at all. Verified live: with only the `dragging` guard above,
		// that catch-up `mouseout` reaches here first, while `dragging` is
		// still false, and deletes the ghost before a drag ever starts --
		// `route_direct` never fired. A real hit-test at the event's own
		// coordinates tells the two cases apart where the event type alone
		// cannot: only actually remove the ghost when the cursor has left
		// both it and the line, not merely because a `mouseout` arrived.
		layer.on('mouseout', (e: LeafletMouseEvent) => {
			if (!ghost || dragging) return;
			const original = e.originalEvent;
			const stillHovering =
				original &&
				(() => {
					const under = document.elementFromPoint(original.clientX, original.clientY);
					return under === ghost!.getElement() || under === layer.getElement();
				})();
			if (stillHovering) return;
			map!.removeLayer(ghost);
			ghost = null;
		});
	}

	/**
	 * Re-route through an edited waypoint list. Works in BOTH modes: a route
	 * is an ordered waypoint list either way, which is what lets a
	 * mis-anchored loop be dragged into shape.
	 */
	async function reroute(waypoints: Waypoint[], insert?: InsertPoint) {
		if (!trip) return;
		// The map already shows the dragged position (the handle followed the
		// pointer); re-fitting bounds on top of that would re-zoom the map for
		// a result the user is already looking at.
		skipFit = true;
		// Editing produces a concrete road route, so an edited loop becomes a
		// direct route — which is exactly the escape hatch the design wants.
		mode = 'direct';
		await runDirect(waypoints, trip.distanceKm, insert);
	}

	// The trip comes from the vehicle the layout activates, which is populated
	// asynchronously — so react to it rather than reading it on mount.
	$effect(() => {
		const vehicle = $activeVehicleStore;
		if (!vehicle || dataLoadStarted) return;
		dataLoadStarted = true;
		void loadTripAndRoute(vehicle.id);
	});

	async function loadTripAndRoute(vehicleId: string) {
		loading = true;
		error = null;
		retryable = true;
		try {
			const trips = await getTrips(vehicleId);
			trip = trips.find((t) => t.id === tripId) ?? null;
			if (!trip) {
				// Not a generation failure — nothing was generated. Retrying would
				// fail identically forever, so this branch offers no retry.
				error = $LL.routeMap.tripNotFound();
				retryable = false;
				return;
			}

			await loadRoute();
		} catch (e) {
			console.error('Failed to load route map:', e);
			error = $LL.routeMap.error();
		} finally {
			loading = false;
		}
	}

	/** The saved-route-or-fresh-plan pipeline, factored out so a retry after a
	 *  failure that happened before `mode` was known (e.g. `getTripRoute`
	 *  itself failing) can re-run the whole decision, not just one mode's path. */
	async function loadRoute() {
		savedRoute = await getTripRoute(tripId);
		if (savedRoute) {
			mode = savedRoute.mode;
			rehydrateEndpoints(savedRoute);
			return;
		}
		await startForTrip();
	}

	/**
	 * A saved route already contains its endpoints — recover them so
	 * Prepočítať and editing work on a re-opened map without re-geocoding.
	 * Without this the resolved state stays null and every re-route request
	 * goes out with an empty waypoint list.
	 */
	function rehydrateEndpoints(route: RouteMap) {
		const points = route.waypoints;
		if (points.length < 2) return;
		const first = points[0];
		const last = points[points.length - 1];
		resolvedOrigin = { lat: first.lat, lon: first.lon, displayName: first.name ?? '' };
		resolvedDestination = { lat: last.lat, lon: last.lon, displayName: last.name ?? '' };
	}

	/** `RouteStart` only returns a `Place` when the book holds both lat and
	 *  lon (see `placed_endpoint` in Rust), so the assertion here reflects
	 *  that invariant rather than guessing. */
	function toEndpoint(place: Place): Endpoint {
		return { lat: place.lat!, lon: place.lon!, displayName: place.displayName };
	}

	/** A shell `Place` for the endpoint the book has no coordinate for yet —
	 *  only `displayName` is real, the rest are values PlaceModal never reads
	 *  for an unplaced entry (its own `canClear` stays false throughout). */
	function unplacedShell(field: 'origin' | 'destination'): Place {
		return {
			displayName: field === 'origin' ? (trip?.origin ?? '') : (trip?.destination ?? ''),
			normalisedName: '',
			uses: 0,
			lat: null,
			lon: null,
			source: null
		};
	}

	/**
	 * Ask the backend what kind of route this row wants, and where its
	 * endpoints are. ONE round trip: mode selection is `mode_for` in Rust, and
	 * both endpoints resolve server-side against the place book.
	 */
	async function startForTrip() {
		if (!trip) return;
		generating = true;
		error = null;
		try {
			const plan = await startRouteForTrip(tripId);
			mode = plan.mode;

			if (plan.mode === 'loop') {
				await runGenerate(trip.distanceKm);
				return;
			}

			resolvedOrigin = plan.origin ? toEndpoint(plan.origin) : null;
			resolvedDestination = plan.destination ? toEndpoint(plan.destination) : null;

			if (!resolvedOrigin) {
				unplacedField = 'origin';
				return;
			}
			if (!resolvedDestination) {
				unplacedField = 'destination';
				return;
			}
			unplacedField = null;

			await runDirect(waypointsFromEndpoints(), trip.distanceKm);
		} catch (e) {
			console.error('Failed to plan trip route:', e);
			if (isMissingEndpointsError(e)) {
				// mode_for's own validation failure for a blank origin or
				// destination -- a real data problem, not retryable, and not
				// the backend's raw English text, which would bypass i18n.
				error = $LL.routeMap.missingEndpoints();
				retryable = false;
			} else {
				// Everything else start_route_for_trip_internal can throw --
				// db.get_trip failing, "Trip not found", list_places_internal
				// failing, or a plain transport error -- is transient. Keep
				// the retryable path, same as every other route-map failure.
				error = $LL.routeMap.routeError();
				retryable = true;
			}
		} finally {
			generating = false;
		}
	}

	/**
	 * The one start_route_for_trip failure that is a genuine data problem
	 * rather than a transient one: `mode_for`'s exact validation text for a
	 * blank origin or destination
	 * (src-tauri/core/src/commands_internal/route_maps.rs:428). Matched
	 * verbatim rather than guessed, per the review that found this catch
	 * previously treated every error as this one.
	 */
	function isMissingEndpointsError(e: unknown): boolean {
		return e instanceof Error && e.message === 'A trip needs both an origin and a destination';
	}

	function waypointsFromEndpoints(): Waypoint[] {
		if (!resolvedOrigin || !resolvedDestination) return [];
		return [
			{ lat: resolvedOrigin.lat, lon: resolvedOrigin.lon, name: resolvedOrigin.displayName },
			{
				lat: resolvedDestination.lat,
				lon: resolvedDestination.lon,
				name: resolvedDestination.displayName
			}
		];
	}

	/**
	 * The place dialog hands back only the coordinate a human confirmed — it
	 * imports no write command itself (PlaceModal's own contract). This page
	 * owns the write, exactly like the Miesta settings page's own handler.
	 * On success the book now has the entry, so re-running `startForTrip`
	 * picks it up and continues to the next unplaced endpoint, or routes.
	 * On failure the dialog is left open (its pin survives) so Save can be
	 * retried — most likely cause is read-only mode, where `save_place` is
	 * always refused.
	 */
	async function handlePlaceSaved(coords: { lat: number; lon: number; source: PlaceSource }) {
		const field = unplacedField;
		if (!field || !trip) return;
		const displayName = field === 'origin' ? trip.origin : trip.destination;
		try {
			await savePlace(displayName, coords.lat, coords.lon, coords.source);
			toast.success($LL.places.saved());
			unplacedField = null;
			await startForTrip();
		} catch (e) {
			console.error('Failed to save place:', e);
			toast.error($LL.places.saveError({ error: String(e) }));
		}
	}

	function closePlaceDialog() {
		unplacedField = null;
	}

	/** Generates and displays a loop route. Persists nothing — only handleSave does. */
	async function runGenerate(targetKm: number) {
		generating = true;
		error = null;
		savedNotice = false;
		try {
			generated = await generateRoute(targetKm);
		} catch (e) {
			console.error('Failed to generate route:', e);
			// Drop the previous proposal: leaving it would let the user save a
			// stale route while an error banner is on screen.
			generated = null;
			error = $LL.routeMap.error();
		} finally {
			generating = false;
		}
	}

	/** Routes and displays a direct route. Persists nothing — only handleSave does. */
	async function runDirect(waypoints: Waypoint[], targetKm: number, insert?: InsertPoint) {
		generating = true;
		error = null;
		savedNotice = false;
		try {
			const routes = await routeDirect(waypoints, targetKm, insert);
			if (routes.length === 0) throw new Error('no routes returned');
			alternatives = routes;
			activeIndex = 0;
			generated = routes[0];
		} catch (e) {
			console.error('Failed to route trip:', e);
			// Same rule as loop mode: drop the proposal so an error banner can
			// never have a stale, saveable route sitting behind it.
			generated = null;
			alternatives = [];
			error = $LL.routeMap.routeError();
		} finally {
			generating = false;
		}
	}

	/** The waypoints any re-route should start from, in either mode. On a
	 *  re-opened saved route these may include vias — always prefer this over
	 *  `waypointsFromEndpoints()`, which drops them. */
	function currentWaypoints(): Waypoint[] {
		return generated?.waypoints ?? savedRoute?.waypoints ?? waypointsFromEndpoints();
	}

	function handleRegenerate() {
		if (!trip) return;
		if (mode === 'loop') {
			void runGenerate(trip.distanceKm);
		} else if (mode === 'direct') {
			void runDirect(currentWaypoints(), trip.distanceKm);
		}
	}

	function handleRetry() {
		error = null;
		if (trip) {
			if (mode === 'loop') {
				void runGenerate(trip.distanceKm);
			} else if (mode === 'direct') {
				void runDirect(currentWaypoints(), trip.distanceKm);
			} else {
				void loadRoute();
			}
			return;
		}
		const vehicle = $activeVehicleStore;
		if (vehicle) void loadTripAndRoute(vehicle.id);
	}

	async function handleSave() {
		if (!generated || !tripId) return;
		saving = true;
		try {
			await saveTripRoute(tripId, generated);
			// Re-read so the displayed route is the persisted one, not a local copy.
			savedRoute = await getTripRoute(tripId);
			generated = null;
			// The alternatives panel guards on `alternatives.length`, not on
			// `generated` -- leaving it populated here would show the picker
			// (and its grey map layers) for a proposal that no longer exists,
			// next to the just-saved route.
			alternatives = [];
			activeIndex = 0;
			announce('route-map-saved');
			savedNotice = true;
			toast.success($LL.routeMap.saved());
		} catch (e) {
			console.error('Failed to save route map:', e);
			toast.error($LL.routeMap.error());
		} finally {
			saving = false;
		}
	}

	async function handleRemoveConfirmed() {
		confirmingRemove = false;
		if (!savedRoute || !tripId) return;
		removing = true;
		try {
			await deleteTripRoute(tripId);
			savedRoute = null;
			savedNotice = false;
			// Unlike handleSave, this never touches `generated` -- Remove only
			// deletes the persisted route, not an in-progress unsaved proposal
			// -- so `alternatives`/`activeIndex` stay in sync with whatever is
			// (or is not) currently generated and need no reset here.
			announce('route-map-removed');
			toast.success($LL.routeMap.removed());
		} catch (e) {
			console.error('Failed to remove route map:', e);
			toast.error($LL.routeMap.error());
		} finally {
			removing = false;
		}
	}

	/** Tells an open logbook tab that this trip's map appeared or disappeared,
	 *  so its row icon does not go stale while both tabs are open. */
	function announce(type: 'route-map-saved' | 'route-map-removed') {
		if (typeof BroadcastChannel === 'undefined') return;
		const channel = new BroadcastChannel('kniha-jazd');
		channel.postMessage({ type, tripId });
		channel.close();
	}

	function formatDeviation(percent: number): string {
		return `${percent >= 0 ? '+' : ''}${percent.toFixed(1)} %`;
	}

	/** Alternatives stay in the backend's fastest-first order (see the brief) -
	 *  this only moves which index is active, never reorders `alternatives`. */
	function selectAlternative(index: number) {
		activeIndex = index;
		generated = alternatives[index];
		skipFit = true;
	}

	function formatDuration(seconds: number): string {
		const total = Math.round(seconds / 60);
		const h = Math.floor(total / 60);
		const m = total % 60;
		return h > 0 ? `${h} h ${m} min` : `${m} min`;
	}
</script>

<div class="map-page" data-test="route-map-page">
	<div class="header">
		<h1>{$LL.routeMap.title()}</h1>
		{#if trip}
			<span class="trip-summary" data-test="trip-summary">
				{trip.origin} → {trip.destination} · {trip.distanceKm} km
			</span>
		{/if}
	</div>

	<div class="toolbar">
		{#if mode === 'loop'}
			<button
				class="button"
				data-test="regenerate-btn"
				onclick={handleRegenerate}
				disabled={busy || !trip}
			>
				{generating ? $LL.routeMap.generating() : $LL.routeMap.regenerate()}
			</button>
		{:else if mode === 'direct'}
			<button
				class="button"
				data-test="recalculate-btn"
				onclick={handleRegenerate}
				disabled={busy || !trip}
			>
				{generating ? $LL.routeMap.generating() : $LL.routeMap.recalculate()}
			</button>
		{/if}
		<button
			class="button secondary"
			data-test="save-btn"
			onclick={handleSave}
			disabled={busy || !generated}
		>
			{$LL.routeMap.save()}
		</button>
		{#if savedRoute}
			<button
				class="button danger"
				data-test="remove-btn"
				onclick={() => (confirmingRemove = true)}
				disabled={busy}
			>
				{$LL.routeMap.remove()}
			</button>
		{/if}
	</div>

	{#if savedNotice}
		<div class="saved-notice" data-test="saved-notice">
			<span>{$LL.routeMap.saved()}</span>
			<button class="button-small" onclick={() => window.close()}>{$LL.common.close()}</button>
		</div>
	{/if}

	{#if error}
		<div class="error-box" data-test="route-map-error">
			<span>{error}</span>
			{#if retryable}
				<button class="button-small" data-test="retry-btn" onclick={handleRetry}>
					{$LL.routeMap.retry()}
				</button>
			{/if}
		</div>
	{/if}

	{#if loading || generating}
		<p class="status" data-test="route-map-status">
			{generating ? $LL.routeMap.generating() : $LL.common.loading()}
		</p>
	{:else if displayRoute}
		<div class="route-info">
			<span class="info-item">
				<span class="label">{$LL.routeMap.targetKm()}</span>
				<span class="value" data-test="target-km">{displayRoute.targetKm.toFixed(1)} km</span>
			</span>
			<span class="info-item">
				<span class="label">{$LL.routeMap.actualKm()}</span>
				<span class="value" data-test="actual-km">{displayRoute.roadKm.toFixed(1)} km</span>
			</span>
			{#if deviationPercent !== null}
				<span class="info-item">
					<span class="label">{$LL.routeMap.deviation()}</span>
					<span class="value" class:off-target={deviationOffTarget} data-test="deviation">
						{formatDeviation(deviationPercent)}
					</span>
				</span>
			{/if}
		</div>
		{#if mode === 'direct' && !hasVias && alternatives.length > 0}
			<div class="alternatives" data-test="alternatives">
				<span class="label">{$LL.routeMap.alternatives()}</span>
				<ul>
					{#each alternatives as route, i}
						<li>
							<button
								class="alternative"
								class:active={i === activeIndex}
								data-test="alternative-btn"
								aria-pressed={i === activeIndex}
								onclick={() => selectAlternative(i)}
							>
								<span>{route.roadKm.toFixed(1)} km</span>
								<span title={$LL.routeMap.duration()}>{formatDuration(route.durationS)}</span>
								<span class:off-target={route.offTarget}>
									{formatDeviation(route.deviationPercent)}
								</span>
							</button>
						</li>
					{/each}
				</ul>
			</div>
		{:else if mode === 'direct' && hasVias}
			<p class="hint" data-test="alternatives-unavailable">
				{$LL.routeMap.alternativesUnavailable()}
			</p>
		{/if}
		{#if stopNames.length > 0}
			<p class="stops" data-test="stops">
				<span class="label">{$LL.routeMap.stops()} ({stopNames.length})</span>
				{stopNames.join(' → ')}
			</p>
		{/if}
		<p class="hint" data-test="edit-hint">{$LL.routeMap.editHint()}</p>
	{/if}

	<div class="map-canvas" bind:this={mapEl} data-test="route-map-canvas"></div>
</div>

{#if confirmingRemove}
	<ConfirmModal
		title={$LL.routeMap.remove()}
		message={$LL.routeMap.confirmRemove()}
		confirmText={$LL.common.delete()}
		danger={true}
		onConfirm={handleRemoveConfirmed}
		onCancel={() => (confirmingRemove = false)}
	/>
{/if}

{#if unplacedField}
	<!-- The key forces a fresh dialog if the target field changes, so a pin
	     from placing the origin cannot survive under the destination's name -
	     PlaceModal seeds its pending coordinate from the prop exactly once. -->
	{#key unplacedField}
		<PlaceModal
			place={unplacedShell(unplacedField)}
			onSave={handlePlaceSaved}
			onClear={closePlaceDialog}
			onClose={closePlaceDialog}
		/>
	{/key}
{/if}

<style>
	.map-page {
		max-width: 1000px;
		margin: 0 auto;
	}

	.header {
		display: flex;
		align-items: baseline;
		gap: 1rem;
		flex-wrap: wrap;
		margin-bottom: 1rem;
	}

	.header h1 {
		margin: 0;
		color: var(--text-primary);
	}

	.trip-summary {
		color: var(--text-secondary);
	}

	.toolbar {
		display: flex;
		gap: 0.5rem;
		margin-bottom: 1rem;
		flex-wrap: wrap;
	}

	.saved-notice {
		display: flex;
		align-items: center;
		gap: 1rem;
		padding: 0.75rem 1rem;
		margin-bottom: 1rem;
		border-radius: 4px;
		background: var(--toast-success-bg);
		color: var(--toast-success-color);
	}

	.error-box {
		display: flex;
		align-items: center;
		gap: 1rem;
		padding: 0.75rem 1rem;
		margin-bottom: 1rem;
		border-radius: 4px;
		background: var(--toast-error-bg);
		color: var(--toast-error-color);
	}

	.status {
		color: var(--text-secondary);
		font-style: italic;
		margin: 0 0 1rem 0;
	}

	.route-info {
		display: flex;
		gap: 1.5rem;
		flex-wrap: wrap;
		margin-bottom: 0.5rem;
	}

	.info-item {
		display: flex;
		align-items: baseline;
		gap: 0.375rem;
	}

	.label {
		color: var(--text-secondary);
		font-size: 0.875rem;
	}

	.value {
		font-weight: 500;
		color: var(--text-primary);
	}

	.value.off-target {
		color: var(--accent-warning-dark);
		font-weight: 600;
	}

	.stops {
		margin: 0 0 1rem 0;
		color: var(--text-primary);
		font-size: 0.875rem;
	}

	.alternatives {
		margin: 0 0 1rem 0;
	}

	.alternatives ul {
		list-style: none;
		margin: 0.375rem 0 0 0;
		padding: 0;
		display: flex;
		flex-direction: column;
		gap: 0.375rem;
	}

	.alternative {
		display: flex;
		width: 100%;
		gap: 1rem;
		align-items: baseline;
		padding: 0.5rem 0.75rem;
		background-color: var(--btn-secondary-bg);
		color: var(--text-primary);
		border: 1px solid var(--border-default);
		border-radius: 4px;
		font-size: 0.875rem;
		text-align: left;
		cursor: pointer;
	}

	.alternative:hover {
		background-color: var(--btn-secondary-hover);
	}

	.alternative.active {
		border-color: var(--btn-active-primary-bg);
		background-color: var(--btn-active-primary-bg);
		color: var(--btn-active-primary-color);
	}

	.alternative .off-target {
		color: var(--accent-warning-dark);
		font-weight: 600;
	}

	/* .alternative.active and .alternative .off-target have equal specificity
	   (two classes each), so source order alone decided the winner --
	   --accent-warning-dark on --btn-active-primary-bg is ~1.1:1 in light
	   theme, effectively invisible. Three classes here always wins, and
	   inheriting the active row's own text color keeps this label exactly as
	   legible as its neighbours in both themes (see theme.css); weight and
	   underline carry the "off target" signal instead of hue. */
	.alternative.active .off-target {
		color: inherit;
		text-decoration: underline;
		text-decoration-thickness: 2px;
	}

	.hint {
		margin: 0 0 1rem 0;
		color: var(--text-secondary);
		font-size: 0.875rem;
		font-style: italic;
	}

	.map-canvas {
		height: 60vh;
		min-height: 360px;
		width: 100%;
		border-radius: 6px;
		border: 1px solid var(--border-default);
	}

	/* Leaflet renders these outside Svelte's tree (a plain divIcon), so the
	   rule must be :global -- scoped styling never reaches them. Supplying
	   `className` on the divIcon replaces Leaflet's own default styling, so
	   the circle, border and shadow below are drawn entirely by this rule. */
	:global(.wp-handle) {
		box-sizing: border-box;
		border-radius: 50%;
		background: var(--accent-primary);
		border: 2px solid var(--bg-surface);
		box-shadow: 0 1px 4px var(--shadow-default);
		cursor: grab;
	}

	/* Endpoints are visually heavier than vias: bigger (set via iconSize) and
	   a thicker border, so the two ends of the trip read as fixed anchors. */
	:global(.wp-endpoint) {
		border-width: 3px;
	}

	.button {
		padding: 0.75rem 1.5rem;
		background-color: var(--btn-active-primary-bg);
		color: var(--btn-active-primary-color);
		border: none;
		border-radius: 4px;
		font-weight: 500;
		cursor: pointer;
		transition: background-color 0.2s;
	}

	.button:hover:not(:disabled) {
		background-color: var(--btn-active-primary-hover);
	}

	.button:disabled {
		opacity: 0.6;
		cursor: not-allowed;
	}

	.button.secondary {
		background-color: var(--btn-active-success-bg);
		color: var(--btn-active-success-color);
	}

	.button.secondary:hover:not(:disabled) {
		background-color: var(--btn-active-success-hover);
	}

	.button.danger {
		background-color: var(--accent-danger-bg);
		color: var(--accent-danger);
	}

	.button.danger:hover:not(:disabled) {
		background-color: var(--accent-danger-hover-bg);
	}

	.button-small {
		padding: 0.375rem 0.875rem;
		background-color: var(--btn-secondary-bg);
		color: var(--text-primary);
		border: none;
		border-radius: 4px;
		font-size: 0.875rem;
		cursor: pointer;
	}

	.button-small:hover {
		background-color: var(--btn-secondary-hover);
	}
</style>

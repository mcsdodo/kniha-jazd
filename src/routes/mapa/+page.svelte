<script lang="ts">
	import 'leaflet/dist/leaflet.css';
	import { onMount, onDestroy } from 'svelte';
	import { page } from '$app/stores';
	import type { Map as LeafletMap, Polyline, Marker, LeafletMouseEvent } from 'leaflet';
	import {
		generateRoute,
		getTripRoute,
		saveTripRoute,
		saveTripRoundTripRoute,
		deleteTripRoute,
		getTrips,
		startRouteForTrip,
		routeDirect,
		routeRoundTrip,
		applyRouteDistance,
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
		PlaceSource,
		Leg,
		LegInsertPoint,
		LegRoute,
		RoundTripRoutes,
		DistanceWriteback
	} from '$lib/types';
	import { activeVehicleStore } from '$lib/stores/vehicles';
	import { toast } from '$lib/stores/toast';
	import ConfirmModal from '$lib/components/ConfirmModal.svelte';
	import PlaceModal from '$lib/components/PlaceModal.svelte';
	import OdometerCascadeModal from '$lib/components/OdometerCascadeModal.svelte';
	import LL from '$lib/i18n/i18n-svelte';

	/** Just enough to route from an endpoint -- not the full place-book `Place`. */
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
	/** Decided by the backend (`mode_for`) -- this page never compares origin
	 *  to destination itself. Null until the first plan or saved route loads. */
	let mode = $state<RouteMode | null>(null);
	/** Alternatives for the current direct route, in the backend's order. */
	let alternatives = $state<GeneratedRoute[]>([]);
	let activeIndex = $state(0);
	/** Direct-mode only (Task 19): ticking appends a return leg back to the
	 *  route's own start. Persisted (Task 20): reopening a saved route sets
	 *  this from `savedRoute.roundTrip` in `loadRoute()`, so Prepočítať
	 *  reproduces the same shape without the user re-ticking the box. */
	let roundTrip = $state(false);
	/** The open (un-closed) waypoint list behind the currently displayed
	 *  direct route -- i.e. `generated.waypoints` with the round-trip closing
	 *  leg stripped back off when one was requested. Used to seed the NEXT
	 *  routeDirect call so toggling the checkbox, regenerating, or dragging a
	 *  handle can never compound the append (each `runDirect` would otherwise
	 *  re-close an already-closed list). Never derived by comparing the first
	 *  and last waypoint -- design decision 5 rules out inferring round-trip
	 *  state from the data; this instead relies on knowing structurally that
	 *  the backend appends exactly one trailing waypoint when asked to. */
	let baseWaypoints = $state<Waypoint[] | null>(null);
	/** Round-trip mode only: both legs as the backend last normalised them,
	 *  with each leg's own alternatives and the table of what every pair adds
	 *  up to. Null in one-way and loop mode. */
	let roundTripRoutes = $state<RoundTripRoutes | null>(null);
	/** Which alternative is chosen on each leg. Independent -- the spec asks
	 *  for a choice per leg, not a choice of pairs. */
	let outboundIndex = $state(0);
	let inboundIndex = $state(0);
	/** The return leg's open waypoint list. `baseWaypoints` holds the outbound
	 *  one in round-trip mode, so the two fields stay a matched pair and
	 *  unticking the checkbox already has the right list to route. */
	let baseInbound = $state<Waypoint[] | null>(null);
	/** The saved route split back into its two legs, kept separately from
	 *  `baseWaypoints`/`baseInbound` so it can act as the FALLBACK when those
	 *  are null.
	 *
	 *  This is load-bearing, not a convenience. `runDirect`'s catch block nulls
	 *  `baseWaypoints` on a failed request, and without this the next
	 *  `currentWaypoints()` would fall through to `savedRoute.waypoints` --
	 *  the CLOSED list. Sent with `round_trip: false`, ADR-041's normaliser
	 *  strips one trailing point off it, which recovered the outbound leg
	 *  before this task and does not any more: a saved `[A, B, v, A]` would
	 *  become the one-way route `A -> B -> v`, moving the return leg's via
	 *  onto the way out. That is the very bug this task removes. */
	let savedLegs = $state<{ outbound: Waypoint[]; inbound: Waypoint[] } | null>(null);
	/** Endpoints as resolved so far, for building waypoint lists. Populated
	 *  only from a fresh plan (`startForTrip`) or a saved route's own
	 *  waypoints (`rehydrateEndpoints`) -- never from the place dialog, so
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
	/** The dry run currently awaiting the user's approval. Nothing is written
	 *  while this is null, and nothing is written when it is dismissed. */
	let writeback = $state<DistanceWriteback | null>(null);
	let applying = $state(false);
	/** The vehicle's trips, so the modal can name the rows the shift moves.
	 *  The plan reports ids only. */
	let yearTrips = $state<Trip[]>([]);

	let mapEl = $state<HTMLDivElement | null>(null);
	let leafletReady = $state(false);
	let mapReady = $state(false);

	// Plain (non-reactive) handles: Leaflet objects are mutable and must never
	// become effect dependencies.
	let leaflet: typeof import('leaflet') | null = null;
	let map: LeafletMap | null = null;
	let routeLayer: Polyline | null = null;
	let inactiveLayers: Polyline[] = [];
	/** Outbound blue, return amber. "Choose per leg" is not usable if the two
	 *  legs are indistinguishable on the map. */
	const OUTBOUND_COLOR = '#0066cc';
	const INBOUND_COLOR = '#d97706';
	const INACTIVE_COLOR = '#94a3b8';
	/** The active line of each leg. Plain handles, like `routeLayer`. */
	let legLayers: Polyline[] = [];
	/** Which line the current ghost was created on. A ghost carries its leg in
	 *  its dragend closure, so moving the cursor from one leg's line to the
	 *  other's must REPLACE it, not move it -- otherwise a via dropped on the
	 *  return leg would be reported against the outbound one. */
	let ghostOwner: Polyline | null = null;
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
	/** Reads `displayRoute`, not just `generated`, so the "alternatives
	 *  unavailable" branch below is reachable whenever the display falls back
	 *  to `savedRoute` -- a cold-loaded saved route, and equally the state
	 *  right after `handleSave` nulls `generated` and re-reads `savedRoute`.
	 *  Previously only reachable right after a fresh `runDirect` proposal
	 *  (task 72's I2 fix only covered the freshly-generated case). Safe for
	 *  loop mode too: both branches that read this are gated on
	 *  `mode === 'direct'`, so a saved loop route's own multi-stop waypoint
	 *  list never lights them up. */
	let hasVias = $derived((displayRoute?.waypoints.length ?? 0) > 2);
	let stopNames = $derived(
		roundTripRoutes
			? [...roundTripRoutes.outboundWaypoints, ...roundTripRoutes.inboundWaypoints.slice(1)]
					.map((w) => w.name)
					.filter((name): name is string => !!name)
			: displayRoute
				? displayRoute.waypoints.map((w) => w.name).filter((name): name is string => !!name)
				: []
	);
	/** The pair currently on screen, as the backend measured it. Null unless a
	 *  round trip has actually been routed. */
	let combinedSelection = $derived(
		roundTripRoutes?.combined?.[outboundIndex]?.[inboundIndex] ?? null
	);
	/** True when there is anything to show: a round trip has no
	 *  `GeneratedRoute`, so `displayRoute` alone would hide the whole panel. */
	let hasRoute = $derived(!!roundTripRoutes || !!displayRoute);
	let displayTargetKm = $derived(roundTripRoutes?.targetKm ?? displayRoute?.targetKm ?? null);
	let displayRoadKm = $derived(combinedSelection?.roadKm ?? displayRoute?.roadKm ?? null);
	// Both come from the backend. The tolerance is a business rule and has one
	// home in Rust (ADR-008); deriving it here would measure road distance
	// against a threshold the algorithm applies to a different quantity.
	let deviationPercent = $derived(
		combinedSelection?.deviationPercent ?? displayRoute?.deviationPercent ?? null
	);
	let deviationOffTarget = $derived(combinedSelection?.offTarget ?? displayRoute?.offTarget ?? false);
	/** Per leg, because the message is per leg now: it appears only where a
	 *  leg genuinely passes through an intermediate stop. A round trip on its
	 *  own no longer triggers it -- that was the point of splitting the
	 *  request. Read off the leg lists, so it is right on a cold load too. */
	let outboundHasVias = $derived((baseWaypoints?.length ?? 0) > 2);
	let inboundHasVias = $derived((baseInbound?.length ?? 0) > 2);
	/** Direct mode cannot route until the book holds a coordinate for both
	 *  ends. Dismissing the place dialog with Escape leaves them unresolved,
	 *  and without this the Recalculate button stayed enabled and its click
	 *  dead-ended in Rust's own "a route needs a start and an end". */
	let endpointsMissing = $derived(
		mode === 'direct' && (!resolvedOrigin || !resolvedDestination)
	);
	let busy = $derived(loading || generating || saving || removing || applying);

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
		const legs = roundTripRoutes;
		const outIndex = outboundIndex;
		const inIndex = inboundIndex;
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
		// A hovering-but-not-yet-dragged ghost is bound to THIS routeLayer
		// instance (attachGhost's mousemove/mouseout listeners live on it).
		// Every draw removes and recreates routeLayer from scratch, even when
		// the coordinates are unchanged, so a live ghost left over from the
		// old instance would survive on the map -- draggable, but anchored to
		// a line that is no longer displayed (or that no longer exists at
		// all). Reachable without ever touching the ghost itself: hover the
		// line, then pick a different alternative (selectAlternative reruns
		// this effect via activeIndex). Clear it unconditionally, same as
		// routeLayer and waypointMarkers below, so the next hover spawns a
		// fresh ghost against whatever line is actually on screen.
		if (ghost) {
			map.removeLayer(ghost);
			ghost = null;
			ghostOwner = null;
		}
		for (const layer of inactiveLayers) {
			map.removeLayer(layer);
		}
		inactiveLayers = [];
		for (const layer of legLayers) {
			map.removeLayer(layer);
		}
		legLayers = [];

		if (roundTripRoutes) {
			drawLegLayers(roundTripRoutes.outbound, outboundIndex, OUTBOUND_COLOR, 'outbound');
			drawLegLayers(roundTripRoutes.inbound, inboundIndex, INBOUND_COLOR, 'inbound');
			if (!shouldSkipFit && legLayers.length > 0) {
				let bounds = legLayers[0].getBounds();
				for (const layer of legLayers.slice(1)) {
					bounds = bounds.extend(layer.getBounds());
				}
				map.fitBounds(bounds, { padding: [30, 30] });
			}
		} else {
			// Inactive alternatives sit UNDER the active line and are clickable.
			alts.forEach((route, i) => {
				if (i === active || route.coordinates.length === 0) return;
				const layer = leaflet!
					.polyline(route.coordinates, { color: INACTIVE_COLOR, weight: 4, opacity: 0.6 })
					.addTo(map!);
				layer.on('click', () => selectAlternative(i));
				inactiveLayers.push(layer);
			});

			if (route && route.coordinates.length > 0) {
				routeLayer = leaflet
					.polyline(route.coordinates, { color: OUTBOUND_COLOR, weight: 5, opacity: 0.85 })
					.addTo(map);
				attachGhost(routeLayer);
				if (!shouldSkipFit) {
					map.fitBounds(routeLayer.getBounds(), { padding: [30, 30] });
				}
			}
		}

		// Runs on every draw, including the empty-route branch above, so
		// handles left over from a route that just got removed (or a load
		// that just failed) do not linger on the map with no line under them.
		drawHandles();
	});

	/** One leg: its unchosen alternatives grey underneath, its chosen line on
	 *  top in `color`. The chosen line carries the ghost handle for that leg. */
	function drawLegLayers(routes: LegRoute[], active: number, color: string, leg: Leg) {
		if (!map || !leaflet) return;
		routes.forEach((route, i) => {
			if (i === active || route.coordinates.length === 0) return;
			const layer = leaflet!
				.polyline(route.coordinates, { color: INACTIVE_COLOR, weight: 4, opacity: 0.6 })
				.addTo(map!);
			layer.on('click', () => selectLeg(leg, i));
			inactiveLayers.push(layer);
		});
		const chosen = routes[active];
		if (!chosen || chosen.coordinates.length === 0) return;
		const layer = leaflet
			.polyline(chosen.coordinates, { color, weight: 5, opacity: 0.85 })
			.addTo(map);
		attachGhost(layer, leg);
		legLayers.push(layer);
	}

	/** Moves which alternative is active on ONE leg. Never reorders the list
	 *  -- that order is the routing service's and is the product decision
	 *  (ADR-038). */
	function selectLeg(leg: Leg, index: number) {
		if (leg === 'outbound') {
			outboundIndex = index;
		} else {
			inboundIndex = index;
		}
		skipFit = true;
	}

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

		if (roundTripRoutes && baseWaypoints && baseInbound) {
			drawLegHandles(baseWaypoints, 'outbound');
			drawLegHandles(baseInbound, 'inbound');
			return;
		}

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

			// Clicking a via removes it. Endpoints are not removable -- that
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
	 * Draggable handles for one leg.
	 *
	 * Only the outbound leg draws endpoints. The return leg's two ends are the
	 * SAME points -- Rust re-joins them on every call -- so drawing them again
	 * would put two handles on one coordinate, and dragging the lower one
	 * would silently be undone by the join.
	 */
	function drawLegHandles(points: Waypoint[], leg: Leg) {
		const legs = (next: Waypoint[]): [Waypoint[], Waypoint[]] =>
			leg === 'outbound' ? [next, currentInbound()] : [currentOutbound(), next];

		points.forEach((wp, i) => {
			const endpoint = i === 0 || i === points.length - 1;
			if (leg === 'inbound' && endpoint) return;

			const marker = leaflet!
				.marker([wp.lat, wp.lon], { draggable: true, icon: handleIcon(leaflet!, endpoint) })
				.addTo(map!);

			// ONE request, on release -- never during the drag.
			marker.on('dragend', () => {
				const { lat, lng } = marker.getLatLng();
				const next = points.map((p, j) => (j === i ? { ...p, lat, lon: lng } : p));
				const [outbound, inbound] = legs(next);
				void rerouteLegs(outbound, inbound);
			});

			if (!endpoint) {
				marker.bindTooltip($LL.routeMap.removeWaypoint());
				marker.on('click', () => {
					const [outbound, inbound] = legs(points.filter((_, j) => j !== i));
					void rerouteLegs(outbound, inbound);
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
	function attachGhost(layer: Polyline, leg?: Leg) {
		if (!map || !leaflet) return;
		layer.on('mousemove', (e: LeafletMouseEvent) => {
			// The cursor crossed from one leg's line to the other's. The
			// existing ghost's dragend closure still names the leg it was
			// created for, so it must be replaced, not moved -- otherwise a
			// via dropped on the return leg is reported against the outbound
			// one, which is the exact bug this task removes.
			if (ghost && ghostOwner !== layer) {
				map!.removeLayer(ghost);
				ghost = null;
				ghostOwner = null;
			}
			if (!ghost) {
				ghost = leaflet!
					.marker(e.latlng, { draggable: true, icon: handleIcon(leaflet!, false) })
					.addTo(map!);
				ghostOwner = layer;
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
					map!.removeLayer(ghost!);
					ghost = null;
					ghostOwner = null;
					if (leg && roundTripRoutes) {
						const polyline =
							leg === 'outbound'
								? roundTripRoutes.outbound[outboundIndex].polyline
								: roundTripRoutes.inbound[inboundIndex].polyline;
						void rerouteLegs(currentOutbound(), currentInbound(), {
							lat,
							lon: lng,
							polyline,
							leg
						});
						return;
					}
					const polyline = generated?.polyline ?? savedRoute?.polyline ?? '';
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
			ghostOwner = null;
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
		// direct route -- which is exactly the escape hatch the design wants.
		mode = 'direct';
		roundTripRoutes = null;
		baseInbound = null;
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
			yearTrips = trips;
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

	/**
	 * Split a saved round trip back into its two legs.
	 *
	 * A slice and a range check, and no rule of its own: `get_trip_route`
	 * resolves the index for every round trip, including the legacy rows that
	 * store none, so there is nothing left to infer here. Deciding how to read
	 * persisted data is business logic and belongs in Rust (ADR-008) -- and
	 * inference in this file is precisely what put a via on the wrong leg
	 * before.
	 *
	 * The two lists overlap by one point on purpose -- the turnaround belongs
	 * to both legs, and both routing requests need it.
	 */
	function splitSavedLegs(route: RouteMap): { outbound: Waypoint[]; inbound: Waypoint[] } | null {
		const at = route.turnaroundIndex;
		const points = route.waypoints;
		if (at === null || at < 1 || at > points.length - 2) return null;
		return { outbound: points.slice(0, at + 1), inbound: points.slice(at) };
	}

	/** The saved-route-or-fresh-plan pipeline, factored out so a retry after a
	 *  failure that happened before `mode` was known (e.g. `getTripRoute`
	 *  itself failing) can re-run the whole decision, not just one mode's path. */
	async function loadRoute() {
		savedRoute = await getTripRoute(tripId);
		if (savedRoute) {
			mode = savedRoute.mode;
			// Restores the checkbox to what was actually saved (Task 20) --
			// without this, Prepočítať would silently hand back a one-way
			// route for a trip the user already marked as a round trip.
			roundTrip = savedRoute.roundTrip;
			const legs = savedRoute.roundTrip ? splitSavedLegs(savedRoute) : null;
			savedLegs = legs;
			rehydrateEndpoints(savedRoute, legs);
			if (legs) {
				baseWaypoints = legs.outbound;
				baseInbound = legs.inbound;
			} else {
				baseWaypoints = savedRoute.roundTrip
					? savedRoute.waypoints.slice(0, -1)
					: savedRoute.waypoints;
				baseInbound = null;
			}
			return;
		}
		await startForTrip();
	}

	/**
	 * A saved route already contains its endpoints -- recover them so
	 * Recalculate and editing work on a re-opened map without re-geocoding.
	 *
	 * A round trip's stored list is closed, so its FIRST and LAST points are
	 * both the origin. Reading the destination off the last point resolves it
	 * to the origin -- wrong before this task, and latent rather than visible:
	 * the only reader is `waypointsFromEndpoints()`, which is the LAST
	 * fallback in `currentWaypoints()` and is never reached while a saved
	 * route is loaded. This task adds a second reader (`endpointsMissing`), so
	 * fix it here rather than leave a wrong value one reader away from
	 * mattering. The outbound leg's own last point is the destination.
	 */
	function rehydrateEndpoints(
		route: RouteMap,
		legs: { outbound: Waypoint[]; inbound: Waypoint[] } | null
	) {
		const points = legs ? legs.outbound : route.waypoints;
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

	/** A shell `Place` for the endpoint the book has no coordinate for yet --
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
	 * The place dialog hands back only the coordinate a human confirmed -- it
	 * imports no write command itself (PlaceModal's own contract). This page
	 * owns the write, exactly like the Miesta settings page's own handler.
	 * On success the book now has the entry, so re-running `startForTrip`
	 * picks it up and continues to the next unplaced endpoint, or routes.
	 * On failure the dialog is left open (its pin survives) so Save can be
	 * retried -- most likely cause is read-only mode, where `save_place` is
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

	/** Generates and displays a loop route. Persists nothing -- only handleSave does. */
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

	/** Routes and displays a direct route. Persists nothing -- only handleSave does.
	 *  `roundTrip` is captured into a local at the top, not read again after the
	 *  await -- the checkbox could otherwise change while the request is in
	 *  flight and this would append (or not) based on a value that no longer
	 *  matches what was actually requested. */
	async function runDirect(waypoints: Waypoint[], targetKm: number, insert?: InsertPoint) {
		generating = true;
		error = null;
		savedNotice = false;
		const closeLoop = roundTrip;
		try {
			const routes = await routeDirect(waypoints, targetKm, insert, closeLoop);
			if (routes.length === 0) throw new Error('no routes returned');
			alternatives = routes;
			activeIndex = 0;
			generated = routes[0];
			// The backend appends exactly one trailing waypoint -- a clone of
			// the route's own (post-insert) first point -- when closeLoop is
			// set. Strip it back off so the next regenerate/insert/drag starts
			// from the open line again; see the `baseWaypoints` doc comment.
			baseWaypoints = closeLoop ? routes[0].waypoints.slice(0, -1) : routes[0].waypoints;
		} catch (e) {
			console.error('Failed to route trip:', e);
			// Same rule as loop mode: drop the proposal so an error banner can
			// never have a stale, saveable route sitting behind it.
			generated = null;
			alternatives = [];
			// Nulling this here is what let a failed request re-expose the
			// "unticked but still N stops" bug (fix round 2, review finding):
			// `currentWaypoints()` falls back to `savedRoute.waypoints`,
			// which can be closed, and the NEXT call could carry
			// `round_trip: false` if the user unticks before retrying. Left
			// as `null` deliberately anyway -- this page does not special-
			// case it, because `route_direct_internal` now normalises the
			// list to match `round_trip` on every call regardless of the
			// shape it receives, so a stale closed fallback here is no
			// longer able to produce a wrong stop count.
			baseWaypoints = null;
			error = $LL.routeMap.routeError();
		} finally {
			generating = false;
		}
	}

	/** Routes and displays a round trip as two legs. Persists nothing -- only
	 *  handleSave does. The returned waypoint lists are adopted wholesale: the
	 *  backend derives the return leg when none is sent and re-joins the two
	 *  ends on every call, so its lists are the only correct ones. */
	async function runRoundTrip(
		outbound: Waypoint[],
		inbound: Waypoint[],
		targetKm: number,
		insert?: LegInsertPoint
	) {
		generating = true;
		error = null;
		savedNotice = false;
		try {
			const routes = await routeRoundTrip(outbound, inbound, targetKm, insert);
			if (routes.outbound.length === 0 || routes.inbound.length === 0) {
				throw new Error('no routes returned');
			}
			roundTripRoutes = routes;
			outboundIndex = 0;
			inboundIndex = 0;
			baseWaypoints = routes.outboundWaypoints;
			baseInbound = routes.inboundWaypoints;
			// A round trip is not a GeneratedRoute. Leaving the one-way state
			// populated would show a stale proposal behind the leg panel and
			// let Save persist the wrong shape.
			generated = null;
			alternatives = [];
			activeIndex = 0;
		} catch (e) {
			console.error('Failed to route the round trip:', e);
			// Same rule as the other two modes: drop the proposal so an error
			// banner can never have a saveable route sitting behind it.
			roundTripRoutes = null;
			error = $LL.routeMap.routeError();
		} finally {
			generating = false;
		}
	}

	/** The outbound leg any re-route starts from. In round-trip mode
	 *  `baseWaypoints` holds the OPEN outbound list, so unticking the checkbox
	 *  already has the right list and needs no stripping. */
	function currentOutbound(): Waypoint[] {
		return baseWaypoints ?? savedLegs?.outbound ?? waypointsFromEndpoints();
	}

	/** The return leg, or an empty list: the backend derives it from the
	 *  outbound leg when it gets nothing. */
	function currentInbound(): Waypoint[] {
		return baseInbound ?? savedLegs?.inbound ?? [];
	}

	/** Re-route after an edit on one leg of a round trip. */
	async function rerouteLegs(outbound: Waypoint[], inbound: Waypoint[], insert?: LegInsertPoint) {
		if (!trip) return;
		// The map already shows the dragged position; re-fitting bounds would
		// re-zoom for a result the user is already looking at.
		skipFit = true;
		mode = 'direct';
		await runRoundTrip(outbound, inbound, trip.distanceKm, insert);
	}

	/** The waypoints any one-way re-route should start from. On a re-opened
	 *  saved route these may include vias -- always prefer this over
	 *  `waypointsFromEndpoints()`, which drops them.
	 *
	 *  `savedLegs?.outbound` sits AHEAD of `savedRoute?.waypoints`: for a
	 *  saved round trip the latter is the closed list, and handing it to
	 *  `route_direct` with `round_trip: false` makes ADR-041's normaliser
	 *  strip one trailing point -- which recovered the outbound leg before
	 *  this task, and now turns `[A, B, v, A]` into the one-way route
	 *  `A -> B -> v`. Reachable without doing anything unusual: untick the
	 *  box on a saved round trip, let the request fail (`runDirect`'s catch
	 *  nulls `baseWaypoints`), then press Retry. */
	function currentWaypoints(): Waypoint[] {
		return baseWaypoints ?? savedLegs?.outbound ?? savedRoute?.waypoints ?? waypointsFromEndpoints();
	}

	function handleRegenerate() {
		if (!trip) return;
		if (mode === 'loop') {
			void runGenerate(trip.distanceKm);
		} else if (mode === 'direct') {
			if (roundTrip) {
				void runRoundTrip(currentOutbound(), currentInbound(), trip.distanceKm);
			} else {
				roundTripRoutes = null;
				baseInbound = null;
				void runDirect(currentWaypoints(), trip.distanceKm);
			}
		}
	}

	function handleRetry() {
		error = null;
		if (trip) {
			if (mode === 'loop') {
				void runGenerate(trip.distanceKm);
			} else if (mode === 'direct') {
				if (roundTrip) {
					void runRoundTrip(currentOutbound(), currentInbound(), trip.distanceKm);
				} else {
					roundTripRoutes = null;
					baseInbound = null;
					void runDirect(currentWaypoints(), trip.distanceKm);
				}
			} else {
				void loadRoute();
			}
			return;
		}
		const vehicle = $activeVehicleStore;
		if (vehicle) void loadTripAndRoute(vehicle.id);
	}

	async function handleSave() {
		if (!tripId) return;
		const legs = roundTripRoutes;
		if (!generated && !legs) return;
		saving = true;
		try {
			if (legs) {
				// Only which alternative was picked crosses the wire. The
				// backend joins the legs, concatenates the geometry and sums
				// the distances (ADR-008).
				await saveTripRoundTripRoute(
					tripId,
					legs.outboundWaypoints,
					legs.inboundWaypoints,
					legs.outbound[outboundIndex].polyline,
					legs.inbound[inboundIndex].polyline,
					legs.outbound[outboundIndex].roadKm,
					legs.inbound[inboundIndex].roadKm,
					legs.targetKm
				);
			} else {
				await saveTripRoute(tripId, generated!, roundTrip);
			}
			// Re-read so the displayed route is the persisted one, not a local copy.
			savedRoute = await getTripRoute(tripId);
			generated = null;
			// The alternatives panel guards on `alternatives.length`, not on
			// `generated` -- leaving it populated here would show the picker
			// (and its grey map layers) for a proposal that no longer exists,
			// next to the just-saved route.
			alternatives = [];
			activeIndex = 0;
			roundTripRoutes = null;
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

	/** Plans the write and opens the modal. Writes NOTHING -- the dry run is
	 *  what fills the warning the user then approves. */
	async function handleApplyDistance() {
		if (!trip || displayRoadKm === null) return;
		applying = true;
		try {
			writeback = await applyRouteDistance(tripId, displayRoadKm, true);
		} catch (e) {
			console.error('Failed to plan the distance write-back:', e);
			toast.error($LL.routeMap.applyDistanceError());
		} finally {
			applying = false;
		}
	}

	/** Writes the distance the user approved -- `writeback.distanceAfter`, not
	 *  whatever the panel shows now: picking a different alternative behind the
	 *  modal must not change what Confirm commits. */
	async function confirmWriteback() {
		const approved = writeback;
		writeback = null;
		if (!approved || !trip) return;
		applying = true;
		try {
			await applyRouteDistance(tripId, approved.distanceAfter, false);
			// The trip's distance IS the map's target, so both the target and
			// the deviation move with it -- re-read the row and the saved map
			// rather than patching the numbers here.
			const vehicle = $activeVehicleStore;
			if (vehicle) {
				const trips = await getTrips(vehicle.id);
				yearTrips = trips;
				trip = trips.find((t) => t.id === tripId) ?? trip;
			}
			savedRoute = await getTripRoute(tripId);
			announce('trip-distance-updated');
			toast.success($LL.routeMap.applyDistanceDone());
		} catch (e) {
			console.error('Failed to write the distance back:', e);
			toast.error($LL.routeMap.applyDistanceError());
		} finally {
			applying = false;
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
	function announce(type: 'route-map-saved' | 'route-map-removed' | 'trip-distance-updated') {
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
				disabled={busy || !trip || endpointsMissing}
			>
				{generating ? $LL.routeMap.generating() : $LL.routeMap.recalculate()}
			</button>
			<label class="round-trip-label" title={$LL.routeMap.roundTripHint()}>
				<input
					type="checkbox"
					data-test="round-trip-checkbox"
					bind:checked={roundTrip}
					onchange={handleRegenerate}
					disabled={busy || !trip || endpointsMissing}
				/>
				{$LL.routeMap.roundTrip()}
			</label>
			<button
				class="button secondary"
				data-test="apply-distance-btn"
				title={$LL.routeMap.applyDistanceTitle()}
				onclick={handleApplyDistance}
				disabled={busy || displayRoadKm === null}
			>
				{$LL.routeMap.applyDistance()}
			</button>
		{/if}
		<button
			class="button secondary"
			data-test="save-btn"
			onclick={handleSave}
			disabled={busy || (!generated && !roundTripRoutes)}
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

	{#if endpointsMissing && !busy}
		<div class="error-box" data-test="endpoint-missing">
			<span>{$LL.routeMap.endpointMissing()}</span>
			<button
				class="button-small"
				data-test="place-endpoint-btn"
				onclick={() => (unplacedField = resolvedOrigin ? 'destination' : 'origin')}
			>
				{$LL.routeMap.placeEndpoint()}
			</button>
		</div>
	{/if}

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
	{:else if hasRoute}
		<div class="route-info">
			<span class="info-item">
				<span class="label">{$LL.routeMap.targetKm()}</span>
				<span class="value" data-test="target-km">{(displayTargetKm ?? 0).toFixed(1)} km</span>
			</span>
			<span class="info-item">
				<span class="label">{$LL.routeMap.actualKm()}</span>
				<span class="value" data-test="actual-km">{(displayRoadKm ?? 0).toFixed(1)} km</span>
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
		{#if mode === 'direct' && roundTrip}
			<div class="alternatives legs" data-test="leg-alternatives">
				<div class="leg" data-test="leg-outbound">
					<span class="label">
						<span class="leg-swatch" style="background-color: {OUTBOUND_COLOR}"></span>
						{$LL.routeMap.alternatives()} - {$LL.routeMap.legOutbound()}
					</span>
					{#if outboundHasVias}
						<p class="hint" data-test="alternatives-unavailable-outbound">
							{$LL.routeMap.alternativesUnavailable()}
						</p>
					{:else if roundTripRoutes}
						<ul>
							{#each roundTripRoutes.outbound as leg, i}
								<li>
									<button
										class="alternative"
										class:active={i === outboundIndex}
										data-test="leg-alternative-btn"
										aria-pressed={i === outboundIndex}
										onclick={() => selectLeg('outbound', i)}
									>
										<span>{leg.roadKm.toFixed(1)} km</span>
										<span title={$LL.routeMap.duration()}>{formatDuration(leg.durationS)}</span>
									</button>
								</li>
							{/each}
						</ul>
					{/if}
				</div>
				<div class="leg" data-test="leg-inbound">
					<span class="label">
						<span class="leg-swatch" style="background-color: {INBOUND_COLOR}"></span>
						{$LL.routeMap.alternatives()} - {$LL.routeMap.legInbound()}
					</span>
					{#if inboundHasVias}
						<p class="hint" data-test="alternatives-unavailable-inbound">
							{$LL.routeMap.alternativesUnavailable()}
						</p>
					{:else if roundTripRoutes}
						<ul>
							{#each roundTripRoutes.inbound as leg, i}
								<li>
									<button
										class="alternative"
										class:active={i === inboundIndex}
										data-test="leg-alternative-btn"
										aria-pressed={i === inboundIndex}
										onclick={() => selectLeg('inbound', i)}
									>
										<span>{leg.roadKm.toFixed(1)} km</span>
										<span title={$LL.routeMap.duration()}>{formatDuration(leg.durationS)}</span>
									</button>
								</li>
							{/each}
						</ul>
					{/if}
				</div>
			</div>
		{:else if mode === 'direct' && !hasVias && alternatives.length > 0}
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

{#if writeback}
	<OdometerCascadeModal
		kind="writeback"
		plan={writeback.plan}
		margin={writeback.margin}
		trips={yearTrips}
		oldDistanceKm={writeback.distanceBefore}
		onConfirm={confirmWriteback}
		onCancel={() => (writeback = null)}
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

	.round-trip-label {
		display: flex;
		align-items: center;
		gap: 0.35rem;
		color: var(--text-primary);
		cursor: pointer;
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

	.leg-swatch {
		display: inline-block;
		width: 0.625rem;
		height: 0.625rem;
		border-radius: 50%;
		margin-right: 0.25rem;
		vertical-align: middle;
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

	.alternatives.legs {
		display: flex;
		gap: 1.5rem;
		flex-wrap: wrap;
	}

	.alternatives.legs .leg {
		flex: 1 1 14rem;
		min-width: 0;
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

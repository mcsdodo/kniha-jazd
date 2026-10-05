//! Tests for the place commands. Task 88 makes a place an entity; the
//! migration tests own the spelling fold, and Task 4 adds the tests for the
//! place write commands.

use super::*;
use std::sync::Mutex;

// ---------------------------------------------------------------------------
// Geocoding a place
// ---------------------------------------------------------------------------

/// A geocoder with no HTTP stack underneath it at all, recording what it was
/// asked.
///
/// This is what `GeocodeProvider` buys. `geocode_tests.rs` already proves the
/// Nominatim client itself, but every one of those tests has to stand up a
/// wiremock server to do it — they exercise the concrete client, not the seam.
/// Answering here proves the command reaches its geocoder through the trait,
/// which is what lets anything other than Nominatim answer it.
struct StubGeocoder {
    answer: Result<Vec<Candidate>, String>,
    asked: Mutex<Vec<String>>,
}

impl StubGeocoder {
    fn answering(candidates: Vec<Candidate>) -> Self {
        Self {
            answer: Ok(candidates),
            asked: Mutex::new(Vec::new()),
        }
    }

    fn failing(message: &str) -> Self {
        Self {
            answer: Err(message.to_string()),
            asked: Mutex::new(Vec::new()),
        }
    }
}

#[async_trait::async_trait]
impl GeocodeProvider for StubGeocoder {
    async fn search(&self, query: &str) -> Result<Vec<Candidate>, String> {
        self.asked.lock().unwrap().push(query.to_string());
        self.answer.clone()
    }
}

#[tokio::test]
async fn geocoding_asks_the_provider_and_returns_its_candidates_in_order() {
    let provider = StubGeocoder::answering(vec![
        Candidate {
            lat: 48.1485965,
            lon: 17.1077477,
            label: "Hlavná stanica, Bratislava, Slovensko".into(),
        },
        Candidate {
            lat: 48.7,
            lon: 21.2,
            label: "Hlavná stanica, Košice, Slovensko".into(),
        },
    ]);

    let candidates = geocode_place_internal(&provider, "Hlavná stanica".to_string())
        .await
        .expect("the stub answers");

    assert_eq!(
        provider.asked.lock().unwrap().as_slice(),
        ["Hlavná stanica"],
        "the address must reach the geocoder verbatim, and exactly once — \
         normalising it here would search for a spelling nobody typed"
    );
    // Best match first. The order is the geocoder's own ranking and the
    // command must hand it on untouched: the dialog preselects the first.
    let labels: Vec<&str> = candidates.iter().map(|c| c.label.as_str()).collect();
    assert_eq!(
        labels,
        [
            "Hlavná stanica, Bratislava, Slovensko",
            "Hlavná stanica, Košice, Slovensko"
        ]
    );
    assert_eq!(candidates[0].lat, 48.1485965);
    assert_eq!(candidates[0].lon, 17.1077477);
}

#[tokio::test]
async fn a_provider_failure_is_an_error_not_an_empty_list() {
    // An empty list already means "place it by hand", so swallowing a failure
    // into one would tell the user the geocoder knows of no such place when in
    // fact it was never asked successfully.
    let provider = StubGeocoder::failing("Geocoding service returned HTTP 429 (Too Many Requests)");

    let err = geocode_place_internal(&provider, "Bratislava".to_string())
        .await
        .expect_err("a provider failure must surface as an error");

    assert_eq!(
        err, "Geocoding service returned HTTP 429 (Too Many Requests)",
        "the provider's own message must reach the caller unrewritten"
    );
}

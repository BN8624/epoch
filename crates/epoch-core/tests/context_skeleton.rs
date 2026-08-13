// M1.4 초기 정치 맥락 — 문화·종교·관계·약속·정보 불변식 통합 테스트

use epoch_core::{
    CULTURE_COUNT, CoreError, HouseRelationKind, INFORMATION_COUNT, InformationConfidence,
    InformationScope, InformationTopic, InitialPoliticalContext, PROMISE_COUNT, RELATION_COUNT,
    RELIGION_COUNT, derive_initial_context, generate_context_world, generate_political_world,
    validate_initial_context,
};
use std::collections::{BTreeMap, BTreeSet};

const SEEDS: [u64; 5] = [0, 1, 2, 42, u64::MAX];

#[test]
fn counts_and_identity_distribution() {
    for seed in SEEDS {
        let cw = generate_context_world(seed).expect("context");
        let c = &cw.context;
        assert_eq!(c.cultures.len(), CULTURE_COUNT, "seed={seed}");
        assert_eq!(c.religions.len(), RELIGION_COUNT, "seed={seed}");
        assert_eq!(c.realm_identities.len(), 6, "seed={seed}");
        assert_eq!(c.house_identities.len(), 18, "seed={seed}");
        assert_eq!(c.person_identities.len(), 144, "seed={seed}");
        assert_eq!(c.relations.len(), RELATION_COUNT, "seed={seed}");
        assert_eq!(c.promises.len(), PROMISE_COUNT, "seed={seed}");
        assert_eq!(c.information.len(), INFORMATION_COUNT, "seed={seed}");

        let mut culture_persons: BTreeMap<&str, usize> = BTreeMap::new();
        let mut religion_persons: BTreeMap<&str, usize> = BTreeMap::new();
        for pi in &c.person_identities {
            *culture_persons.entry(pi.culture_id.as_str()).or_insert(0) += 1;
            *religion_persons.entry(pi.religion_id.as_str()).or_insert(0) += 1;
        }
        assert_eq!(
            culture_persons.get("culture-amber"),
            Some(&48),
            "seed={seed}"
        );
        assert_eq!(
            culture_persons.get("culture-river"),
            Some(&48),
            "seed={seed}"
        );
        assert_eq!(
            culture_persons.get("culture-stone"),
            Some(&48),
            "seed={seed}"
        );
        assert_eq!(
            religion_persons.get("faith-solar"),
            Some(&72),
            "seed={seed}"
        );
        assert_eq!(
            religion_persons.get("faith-ancestral"),
            Some(&72),
            "seed={seed}"
        );

        let mut culture_houses: BTreeMap<&str, usize> = BTreeMap::new();
        let mut religion_houses: BTreeMap<&str, usize> = BTreeMap::new();
        for hi in &c.house_identities {
            *culture_houses.entry(hi.culture_id.as_str()).or_insert(0) += 1;
            *religion_houses.entry(hi.religion_id.as_str()).or_insert(0) += 1;
        }
        assert_eq!(culture_houses.get("culture-amber"), Some(&6), "seed={seed}");
        assert_eq!(culture_houses.get("culture-river"), Some(&6), "seed={seed}");
        assert_eq!(culture_houses.get("culture-stone"), Some(&6), "seed={seed}");
        assert_eq!(religion_houses.get("faith-solar"), Some(&9), "seed={seed}");
        assert_eq!(
            religion_houses.get("faith-ancestral"),
            Some(&9),
            "seed={seed}"
        );
    }
}

#[test]
fn person_identity_inherits_house() {
    for seed in SEEDS {
        let cw = generate_context_world(seed).expect("context");
        let house_by_id: BTreeMap<_, _> = cw
            .context
            .house_identities
            .iter()
            .map(|h| (h.house_id.as_str(), h))
            .collect();
        let person_house: BTreeMap<_, _> = cw
            .political
            .dynastic
            .population
            .persons
            .iter()
            .map(|p| (p.id.as_str(), p.house_id.as_str()))
            .collect();
        for pi in &cw.context.person_identities {
            let hid = person_house[pi.person_id.as_str()];
            let hi = house_by_id[hid];
            assert_eq!(
                pi.culture_id, hi.culture_id,
                "seed={seed} person={}",
                pi.person_id
            );
            assert_eq!(
                pi.religion_id, hi.religion_id,
                "seed={seed} person={}",
                pi.person_id
            );
        }
    }
}

#[test]
fn relations_canonical_unique_intra_cross() {
    for seed in SEEDS {
        let cw = generate_context_world(seed).expect("context");
        let house_realm: BTreeMap<_, _> = cw
            .political
            .dynastic
            .population
            .houses
            .iter()
            .map(|h| (h.id.as_str(), h.realm_id.as_str()))
            .collect();
        let mut pairs = BTreeSet::new();
        let mut intra = 0usize;
        let mut cross = 0usize;
        for rel in &cw.context.relations {
            assert!(rel.house_a_id < rel.house_b_id, "seed={seed} non-canonical");
            assert!(
                pairs.insert((rel.house_a_id.as_str(), rel.house_b_id.as_str())),
                "seed={seed} duplicate pair"
            );
            if house_realm[rel.house_a_id.as_str()] == house_realm[rel.house_b_id.as_str()] {
                intra += 1;
            } else {
                cross += 1;
                assert_eq!(rel.kind, HouseRelationKind::Cooperative);
            }
        }
        assert_eq!(intra, 18, "seed={seed}");
        assert_eq!(cross, 6, "seed={seed}");
        assert_eq!(pairs.len(), 24, "seed={seed}");
    }
}

#[test]
fn promises_per_realm_share_reward() {
    for seed in SEEDS {
        let cw = generate_context_world(seed).expect("context");
        let mut by_realm: BTreeMap<&str, Vec<_>> = BTreeMap::new();
        for p in &cw.context.promises {
            by_realm.entry(p.realm_id.as_str()).or_default().push(p);
        }
        assert_eq!(by_realm.len(), 6, "seed={seed}");
        let mut reward_keys = BTreeSet::new();
        for (realm, list) in &by_realm {
            assert_eq!(list.len(), 2, "seed={seed} realm={realm}");
            assert_eq!(list[0].reward_key, list[1].reward_key, "seed={seed}");
            assert_eq!(
                list[0].reward_key,
                format!("reward:{realm}:council-seat"),
                "seed={seed}"
            );
            assert_ne!(list[0].promisee_person_id, list[1].promisee_person_id);
            assert_eq!(list[0].promisor_person_id, list[1].promisor_person_id);
            assert!(reward_keys.insert(list[0].reward_key.as_str()));
            for p in list {
                let mut expected =
                    vec![p.promisor_person_id.as_str(), p.promisee_person_id.as_str()];
                expected.sort();
                let actual: Vec<_> = p.known_by_person_ids.iter().map(|s| s.as_str()).collect();
                assert_eq!(actual, expected, "seed={seed} promise={}", p.id);
            }
        }
    }
}

#[test]
fn information_scope_confidence_and_asymmetry() {
    for seed in SEEDS {
        let cw = generate_context_world(seed).expect("context");
        let mut public_confirmed = 0usize;
        let mut private_confirmed = 0usize;
        let mut private_unverified = 0usize;
        for item in &cw.context.information {
            match (item.topic, item.scope, item.confidence) {
                (
                    InformationTopic::ReligiousMinority,
                    InformationScope::Public,
                    InformationConfidence::Confirmed,
                ) => {
                    public_confirmed += 1;
                    assert!(item.known_by_person_ids.is_empty());
                }
                (
                    InformationTopic::PromiseConflict,
                    InformationScope::Private,
                    InformationConfidence::Confirmed,
                ) => {
                    private_confirmed += 1;
                    assert_eq!(item.known_by_person_ids.len(), 2);
                }
                (
                    InformationTopic::PromiseConflict,
                    InformationScope::Private,
                    InformationConfidence::Unverified,
                ) => {
                    private_unverified += 1;
                    assert_eq!(item.known_by_person_ids.len(), 1);
                }
                other => panic!("seed={seed} unexpected info {:?}", other),
            }
        }
        assert_eq!(public_confirmed, 6, "seed={seed}");
        assert_eq!(private_confirmed, 6, "seed={seed}");
        assert_eq!(private_unverified, 6, "seed={seed}");
    }
}

#[test]
fn seed1_realm01_asymmetry_concrete() {
    let cw = generate_context_world(1).expect("context");
    let houses: Vec<_> = cw
        .political
        .dynastic
        .population
        .houses
        .iter()
        .filter(|h| h.realm_id == "realm-01")
        .collect();
    let mut houses = houses;
    houses.sort_by(|a, b| a.id.cmp(&b.id));
    assert_eq!(houses.len(), 3);
    let h0 = houses[0];
    let h1 = houses[1];
    let h2 = houses[2];

    let house_id: BTreeMap<_, _> = cw
        .context
        .house_identities
        .iter()
        .map(|h| (h.house_id.as_str(), h))
        .collect();
    let i0 = house_id[h0.id.as_str()];
    let i1 = house_id[h1.id.as_str()];
    let i2 = house_id[h2.id.as_str()];

    // realm-01: amber + solar majority
    assert_eq!(i0.culture_id, "culture-amber");
    assert_eq!(i0.religion_id, "faith-solar");
    // religious minority
    assert_eq!(i1.culture_id, "culture-amber");
    assert_eq!(i1.religion_id, "faith-ancestral");
    // cultural minority
    assert_eq!(i2.culture_id, "culture-river");
    assert_eq!(i2.religion_id, "faith-solar");

    // relations
    let find_rel = |a: &str, b: &str| {
        let (x, y) = if a < b { (a, b) } else { (b, a) };
        cw.context
            .relations
            .iter()
            .find(|r| r.house_a_id == x && r.house_b_id == y)
            .expect("relation")
    };
    assert_eq!(find_rel(&h0.id, &h1.id).kind, HouseRelationKind::Rival);
    assert_eq!(
        find_rel(&h0.id, &h2.id).kind,
        HouseRelationKind::Cooperative
    );
    assert_eq!(
        find_rel(&h1.id, &h2.id).kind,
        HouseRelationKind::Competitive
    );

    // promises: same reward to both non-ruling heads
    let promises: Vec<_> = cw
        .context
        .promises
        .iter()
        .filter(|p| p.realm_id == "realm-01")
        .collect();
    assert_eq!(promises.len(), 2);
    assert_eq!(promises[0].reward_key, promises[1].reward_key);
    assert_eq!(promises[0].reward_key, "reward:realm-01:council-seat");
    assert_eq!(promises[0].promisor_person_id, h0.head_person_id);
    assert_eq!(promises[1].promisor_person_id, h0.head_person_id);
    let promisees: BTreeSet<_> = promises
        .iter()
        .map(|p| p.promisee_person_id.as_str())
        .collect();
    assert_eq!(
        promisees,
        BTreeSet::from([h1.head_person_id.as_str(), h2.head_person_id.as_str()])
    );

    // confirmed conflict: ruler + member_ids[3]
    let confirmed = cw
        .context
        .information
        .iter()
        .find(|i| {
            i.realm_id == "realm-01"
                && i.topic == InformationTopic::PromiseConflict
                && i.confidence == InformationConfidence::Confirmed
        })
        .expect("confirmed");
    let rhc = h0.member_ids.get(3).expect("member_ids[3]");
    let mut expected_confirmed = vec![h0.head_person_id.as_str(), rhc.as_str()];
    expected_confirmed.sort();
    let actual_confirmed: Vec<_> = confirmed
        .known_by_person_ids
        .iter()
        .map(|s| s.as_str())
        .collect();
    assert_eq!(actual_confirmed, expected_confirmed);

    // unverified: house 2 (local[1]) head only
    let unverified = cw
        .context
        .information
        .iter()
        .find(|i| {
            i.realm_id == "realm-01"
                && i.topic == InformationTopic::PromiseConflict
                && i.confidence == InformationConfidence::Unverified
        })
        .expect("unverified");
    assert_eq!(
        unverified.known_by_person_ids,
        vec![h1.head_person_id.clone()]
    );

    // house 3 head does not know conflict information
    let conflict_knowers: BTreeSet<_> = cw
        .context
        .information
        .iter()
        .filter(|i| i.realm_id == "realm-01" && i.topic == InformationTopic::PromiseConflict)
        .flat_map(|i| i.known_by_person_ids.iter().map(|s| s.as_str()))
        .collect();
    assert!(!conflict_knowers.contains(h2.head_person_id.as_str()));

    // house 3 head still knows own promise only
    let own_promise = promises
        .iter()
        .find(|p| p.promisee_person_id == h2.head_person_id)
        .expect("house3 promise");
    assert!(
        own_promise
            .known_by_person_ids
            .iter()
            .any(|k| k == &h2.head_person_id)
    );
}

#[test]
fn same_seed_structure_and_bytes_equal() {
    let a = generate_context_world(1).expect("a");
    let b = generate_context_world(1).expect("b");
    assert_eq!(a, b);
    assert_eq!(
        a.to_compact_json_bytes().unwrap(),
        b.to_compact_json_bytes().unwrap()
    );
    let c = generate_context_world(2).expect("c");
    assert_ne!(
        a.to_compact_json_bytes().unwrap(),
        c.to_compact_json_bytes().unwrap()
    );
}

#[test]
fn seed_difference_and_schema() {
    let a = generate_context_world(1).expect("1");
    let b = generate_context_world(2).expect("2");
    assert_ne!(a, b);
    assert_eq!(a.schema_version, 1);
    assert_eq!(a.seed, 1);
    assert_eq!(b.seed, 2);
}

#[test]
fn malformed_context_returns_error_no_panic() {
    let political = generate_political_world(1).expect("political");
    let empty = InitialPoliticalContext {
        cultures: vec![],
        religions: vec![],
        realm_identities: vec![],
        house_identities: vec![],
        person_identities: vec![],
        relations: vec![],
        promises: vec![],
        information: vec![],
    };
    let err = validate_initial_context(&political, &empty).unwrap_err();
    assert!(matches!(err, CoreError::InvalidContext(_)));

    // derive on valid political still works
    let ok = derive_initial_context(&political).expect("derive");
    assert_eq!(ok.cultures.len(), 3);
}

#[test]
fn public_private_information_contract() {
    let cw = generate_context_world(1).expect("context");
    for item in &cw.context.information {
        match item.scope {
            InformationScope::Public => assert!(
                item.known_by_person_ids.is_empty(),
                "public {} must be empty known_by",
                item.id
            ),
            InformationScope::Private => assert!(
                !item.known_by_person_ids.is_empty(),
                "private {} must have knowers",
                item.id
            ),
        }
    }
}

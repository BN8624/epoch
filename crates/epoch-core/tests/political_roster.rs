// M1.3 정치 활동 계층 — Active 24 / Supporting 120 불변식 통합 테스트

use epoch_core::{
    ACTIVE_ACTOR_COUNT, ActivationReason, ActiveRole, CoreError, GenerationBand, PERSON_COUNT,
    SUPPORTING_PERSON_COUNT, derive_political_roster, generate_dynastic_world,
    generate_political_world, generate_world, validate_political_roster, validate_population,
    validate_world,
};
use std::collections::{BTreeMap, BTreeSet};

const SEEDS: [u64; 5] = [0, 1, 2, 42, u64::MAX];

#[test]
fn active_supporting_counts_and_roles() {
    for seed in SEEDS {
        let pw = generate_political_world(seed).expect("political");
        assert_eq!(pw.roster.active_actors.len(), ACTIVE_ACTOR_COUNT);
        assert_eq!(
            pw.roster.supporting_person_ids.len(),
            SUPPORTING_PERSON_COUNT
        );

        let mut rulers = 0usize;
        let mut heads = 0usize;
        let mut rhc = 0usize;
        for a in &pw.roster.active_actors {
            match a.primary_role {
                ActiveRole::Ruler => rulers += 1,
                ActiveRole::HouseHead => heads += 1,
                ActiveRole::RulingHouseCurrent => rhc += 1,
            }
        }
        assert_eq!(rulers, 6, "seed={seed}");
        assert_eq!(heads, 12, "seed={seed}");
        assert_eq!(rhc, 6, "seed={seed}");
    }
}

#[test]
fn realm_distribution_4_active_20_supporting() {
    for seed in SEEDS {
        let pw = generate_political_world(seed).expect("political");
        let mut active_by_realm: BTreeMap<String, usize> = BTreeMap::new();
        for a in &pw.roster.active_actors {
            *active_by_realm.entry(a.realm_id.clone()).or_insert(0) += 1;
        }
        let supporting: BTreeSet<_> = pw.roster.supporting_person_ids.iter().cloned().collect();
        let mut supporting_by_realm: BTreeMap<String, usize> = BTreeMap::new();
        for p in &pw.dynastic.population.persons {
            if supporting.contains(&p.id) {
                *supporting_by_realm.entry(p.realm_id.clone()).or_insert(0) += 1;
            }
        }
        assert_eq!(pw.dynastic.world.realms.len(), 6);
        for realm in &pw.dynastic.world.realms {
            assert_eq!(
                *active_by_realm.get(&realm.id).unwrap_or(&0),
                4,
                "seed={seed} realm={}",
                realm.id
            );
            assert_eq!(
                *supporting_by_realm.get(&realm.id).unwrap_or(&0),
                20,
                "seed={seed} realm={}",
                realm.id
            );
        }
    }
}

#[test]
fn all_active_are_current_generation() {
    for seed in SEEDS {
        let pw = generate_political_world(seed).expect("political");
        let by_id: BTreeMap<_, _> = pw
            .dynastic
            .population
            .persons
            .iter()
            .map(|p| (p.id.as_str(), p))
            .collect();
        for a in &pw.roster.active_actors {
            let p = by_id[a.person_id.as_str()];
            assert_eq!(
                p.generation,
                GenerationBand::Current,
                "seed={seed} person={}",
                a.person_id
            );
        }
    }
}

#[test]
fn ruler_links_and_heads_and_member3_are_active() {
    for seed in SEEDS {
        let pw = generate_political_world(seed).expect("political");
        let active: BTreeSet<_> = pw
            .roster
            .active_actors
            .iter()
            .map(|a| a.person_id.as_str())
            .collect();

        // all 6 ruler-linked
        assert_eq!(pw.dynastic.population.ruler_links.len(), 6);
        for link in &pw.dynastic.population.ruler_links {
            assert!(
                active.contains(link.person_id.as_str()),
                "seed={seed} ruler person {} not active",
                link.person_id
            );
        }

        // houses by realm
        let mut by_realm: BTreeMap<String, Vec<_>> = BTreeMap::new();
        for h in &pw.dynastic.population.houses {
            by_realm.entry(h.realm_id.clone()).or_default().push(h);
        }
        for houses in by_realm.values_mut() {
            houses.sort_by(|a, b| a.id.cmp(&b.id));
        }

        let mut non_ruling_heads = 0usize;
        let mut member3 = 0usize;
        for houses in by_realm.values() {
            assert_eq!(houses.len(), 3);
            // ruling member_ids[3]
            let mid3 = &houses[0].member_ids[3];
            assert!(
                active.contains(mid3.as_str()),
                "seed={seed} member_ids[3] {mid3} not active"
            );
            member3 += 1;
            for house in houses.iter().skip(1) {
                assert!(
                    active.contains(house.head_person_id.as_str()),
                    "seed={seed} non-ruling head {} not active",
                    house.head_person_id
                );
                non_ruling_heads += 1;
            }
        }
        assert_eq!(non_ruling_heads, 12);
        assert_eq!(member3, 6);
    }
}

#[test]
fn coverage_disjoint_and_unique() {
    for seed in SEEDS {
        let pw = generate_political_world(seed).expect("political");
        let active: BTreeSet<_> = pw
            .roster
            .active_actors
            .iter()
            .map(|a| a.person_id.clone())
            .collect();
        let supporting: BTreeSet<_> = pw.roster.supporting_person_ids.iter().cloned().collect();
        assert_eq!(active.len(), ACTIVE_ACTOR_COUNT);
        assert_eq!(supporting.len(), SUPPORTING_PERSON_COUNT);
        assert!(active.is_disjoint(&supporting));
        let all: BTreeSet<_> = active.union(&supporting).cloned().collect();
        let persons: BTreeSet<_> = pw
            .dynastic
            .population
            .persons
            .iter()
            .map(|p| p.id.clone())
            .collect();
        assert_eq!(all, persons);
        assert_eq!(all.len(), PERSON_COUNT);
    }
}

#[test]
fn ruler_activation_reasons_include_house_head() {
    let pw = generate_political_world(1).expect("political");
    for a in &pw.roster.active_actors {
        match a.primary_role {
            ActiveRole::Ruler => {
                assert_eq!(
                    a.activation_reasons,
                    vec![ActivationReason::Ruler, ActivationReason::HouseHead]
                );
            }
            ActiveRole::HouseHead => {
                assert_eq!(a.activation_reasons, vec![ActivationReason::HouseHead]);
            }
            ActiveRole::RulingHouseCurrent => {
                assert_eq!(
                    a.activation_reasons,
                    vec![ActivationReason::RulingHouseCurrent]
                );
            }
        }
    }
}

#[test]
fn ordering_is_deterministic_by_person_id() {
    let pw = generate_political_world(42).expect("political");
    let mut prev = "";
    for a in &pw.roster.active_actors {
        assert!(a.person_id.as_str() >= prev);
        prev = a.person_id.as_str();
    }
    prev = "";
    for sid in &pw.roster.supporting_person_ids {
        assert!(sid.as_str() >= prev);
        prev = sid.as_str();
    }
}

#[test]
fn same_seed_equality_and_seed_1_2_differ() {
    let a = generate_political_world(1).expect("a");
    let b = generate_political_world(1).expect("b");
    assert_eq!(a, b);
    let ba = a.to_compact_json_bytes().unwrap();
    let bb = b.to_compact_json_bytes().unwrap();
    assert_eq!(ba, bb);

    let c = generate_political_world(2).expect("c");
    assert_ne!(a, c);
    assert_ne!(ba, c.to_compact_json_bytes().unwrap());
}

#[test]
fn political_does_not_change_population_or_world_bytes() {
    for seed in [1u64, 2] {
        let world = generate_world(seed).expect("world");
        let dynastic = generate_dynastic_world(seed).expect("dynastic");
        let political = generate_political_world(seed).expect("political");

        assert_eq!(world, political.dynastic.world);
        assert_eq!(dynastic, political.dynastic);
        assert_eq!(
            world.to_compact_json_bytes().unwrap(),
            political.dynastic.world.to_compact_json_bytes().unwrap()
        );
        assert_eq!(
            dynastic.to_compact_json_bytes().unwrap(),
            political.dynastic.to_compact_json_bytes().unwrap()
        );
        validate_world(&political.dynastic.world).expect("world ok");
        validate_population(&political.dynastic.world, &political.dynastic.population)
            .expect("pop ok");
        validate_political_roster(&political.dynastic, &political.roster).expect("roster ok");
    }
}

#[test]
fn invalid_roster_rejected() {
    let d = generate_dynastic_world(1).expect("dynastic");
    let mut roster = derive_political_roster(&d).expect("roster");
    // drop one active → break counts
    roster.active_actors.pop();
    let err = validate_political_roster(&d, &roster).expect_err("must fail");
    match err {
        CoreError::InvalidPolitical(msg) => {
            assert!(
                msg.contains("active count") || msg.contains("coverage"),
                "msg={msg}"
            );
        }
        other => panic!("unexpected error: {other:?}"),
    }

    // duplicate supporting
    let mut roster2 = derive_political_roster(&d).expect("roster2");
    roster2.supporting_person_ids[0] = roster2.supporting_person_ids[1].clone();
    let err2 = validate_political_roster(&d, &roster2).expect_err("must fail dup");
    assert!(matches!(err2, CoreError::InvalidPolitical(_)));

    // malformed dynastic (short member_ids) must fail closed, not panic
    let mut d_bad = generate_dynastic_world(1).expect("dynastic bad");
    let roster_ok = derive_political_roster(&d_bad).expect("roster ok");
    d_bad.population.houses[0].member_ids.truncate(1);
    let err3 = validate_political_roster(&d_bad, &roster_ok).expect_err("must not panic");
    assert!(matches!(err3, CoreError::InvalidPolitical(_)));
}

#[test]
fn role_invariants_match_structure() {
    let pw = generate_political_world(1).expect("political");
    let person_by_id: BTreeMap<_, _> = pw
        .dynastic
        .population
        .persons
        .iter()
        .map(|p| (p.id.as_str(), p))
        .collect();
    let ruler_ids: BTreeSet<_> = pw
        .dynastic
        .population
        .ruler_links
        .iter()
        .map(|l| l.person_id.as_str())
        .collect();
    let mut houses_by_realm: BTreeMap<String, Vec<_>> = BTreeMap::new();
    for h in &pw.dynastic.population.houses {
        houses_by_realm
            .entry(h.realm_id.clone())
            .or_default()
            .push(h);
    }
    for houses in houses_by_realm.values_mut() {
        houses.sort_by(|a, b| a.id.cmp(&b.id));
    }

    for a in &pw.roster.active_actors {
        let p = person_by_id[a.person_id.as_str()];
        assert_eq!(p.realm_id, a.realm_id);
        match a.primary_role {
            ActiveRole::Ruler => {
                assert!(ruler_ids.contains(a.person_id.as_str()));
                let houses = &houses_by_realm[&a.realm_id];
                assert_eq!(houses[0].head_person_id, a.person_id);
            }
            ActiveRole::HouseHead => {
                assert!(!ruler_ids.contains(a.person_id.as_str()));
                let house = pw
                    .dynastic
                    .population
                    .houses
                    .iter()
                    .find(|h| h.head_person_id == a.person_id)
                    .expect("head house");
                assert_eq!(house.realm_id, a.realm_id);
                let houses = &houses_by_realm[&a.realm_id];
                assert_ne!(houses[0].id, house.id);
            }
            ActiveRole::RulingHouseCurrent => {
                let houses = &houses_by_realm[&a.realm_id];
                assert_eq!(houses[0].member_ids[3], a.person_id);
                assert_ne!(houses[0].head_person_id, a.person_id);
                assert!(!ruler_ids.contains(a.person_id.as_str()));
            }
        }
    }
}

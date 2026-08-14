// M1.5 초기 계승 권리 — 직계·복권 fixture 불변식 통합 테스트

use epoch_core::{
    ActiveRole, CONTEXT_WORLD_SCHEMA_VERSION, ClaimBasis, ClaimStanding, CoreError,
    DYNASTIC_WORLD_SCHEMA_VERSION, GenerationBand, InitialRights, POLITICAL_WORLD_SCHEMA_VERSION,
    REALM_RIGHTS_COUNT, RIGHT_EVIDENCE_COUNT, RIGHTS_WORLD_SCHEMA_VERSION, SAVE_SCHEMA_VERSION,
    SUCCESSION_CLAIM_COUNT, SuccessionClaim, WORLD_SCHEMA_VERSION, derive_initial_rights,
    generate_context_world, generate_dynastic_world, generate_rights_world,
    validate_initial_rights,
};
use std::collections::{BTreeMap, BTreeSet};

mod common;

const SEEDS: [u64; 5] = [0, 1, 2, 42, u64::MAX];

fn houses_sorted_for_realm<'a>(
    world: &'a epoch_core::RightsWorld,
    realm_id: &str,
) -> Vec<&'a epoch_core::population::House> {
    let mut houses: Vec<_> = world
        .context_world
        .political
        .dynastic
        .population
        .houses
        .iter()
        .filter(|h| h.realm_id == realm_id)
        .collect();
    houses.sort_by(|a, b| a.id.cmp(&b.id));
    houses
}

fn person_by_id(
    world: &epoch_core::RightsWorld,
) -> BTreeMap<&str, &epoch_core::population::Person> {
    world
        .context_world
        .political
        .dynastic
        .population
        .persons
        .iter()
        .map(|p| (p.id.as_str(), p))
        .collect()
}

fn house_identity_by_id(
    world: &epoch_core::RightsWorld,
) -> BTreeMap<&str, &epoch_core::HouseIdentity> {
    world
        .context_world
        .context
        .house_identities
        .iter()
        .map(|h| (h.house_id.as_str(), h))
        .collect()
}

fn realm_identity_by_id(
    world: &epoch_core::RightsWorld,
) -> BTreeMap<&str, &epoch_core::RealmIdentity> {
    world
        .context_world
        .context
        .realm_identities
        .iter()
        .map(|r| (r.realm_id.as_str(), r))
        .collect()
}

#[test]
fn counts_basis_standing_and_per_realm_shape() {
    for seed in SEEDS {
        let rw = generate_rights_world(seed).expect("rights");
        let rights = &rw.rights;
        assert_eq!(rights.realms.len(), REALM_RIGHTS_COUNT, "seed={seed}");
        assert_eq!(rights.claims.len(), SUCCESSION_CLAIM_COUNT, "seed={seed}");
        assert_eq!(
            rights.evidence_records.len(),
            RIGHT_EVIDENCE_COUNT,
            "seed={seed}"
        );

        let mut direct = 0usize;
        let mut restored = 0usize;
        let mut strong = 0usize;
        let mut contested = 0usize;
        for claim in &rights.claims {
            match claim.basis {
                ClaimBasis::DirectDescent => direct += 1,
                ClaimBasis::RestoredLineRecord => restored += 1,
            }
            match claim.standing {
                ClaimStanding::Strong => strong += 1,
                ClaimStanding::Contested => contested += 1,
            }
        }
        assert_eq!(direct, 6, "seed={seed}");
        assert_eq!(restored, 6, "seed={seed}");
        assert_eq!(strong, 6, "seed={seed}");
        assert_eq!(contested, 6, "seed={seed}");

        let mut claims_by_realm: BTreeMap<&str, Vec<&SuccessionClaim>> = BTreeMap::new();
        for claim in &rights.claims {
            claims_by_realm
                .entry(claim.realm_id.as_str())
                .or_default()
                .push(claim);
        }
        assert_eq!(claims_by_realm.len(), 6, "seed={seed}");
        for (realm_id, list) in &claims_by_realm {
            assert_eq!(list.len(), 2, "seed={seed} realm={realm_id}");
        }
        for rr in &rights.realms {
            assert_eq!(rr.claim_ids.len(), 2, "seed={seed} realm={}", rr.realm_id);
        }
        assert_eq!(rw.schema_version, RIGHTS_WORLD_SCHEMA_VERSION);
        assert_eq!(rw.seed, seed);
    }
}

#[test]
fn direct_parent_graph_and_restored_evidence() {
    for seed in SEEDS {
        let rw = generate_rights_world(seed).expect("rights");
        let persons = person_by_id(&rw);
        let house_id = house_identity_by_id(&rw);
        let realm_id = realm_identity_by_id(&rw);
        let claim_by_id: BTreeMap<_, _> = rw
            .rights
            .claims
            .iter()
            .map(|c| (c.id.as_str(), c))
            .collect();
        let rec_by_id: BTreeMap<_, _> = rw
            .rights
            .evidence_records
            .iter()
            .map(|r| (r.id.as_str(), r))
            .collect();

        for rr in &rw.rights.realms {
            let houses = houses_sorted_for_realm(&rw, &rr.realm_id);
            assert_eq!(houses.len(), 3, "seed={seed}");
            let ruling = houses[0];
            let cultural = houses[2];
            let incumbent = persons[rr.incumbent_person_id.as_str()];
            assert_eq!(incumbent.id, ruling.head_person_id);
            assert_eq!(incumbent.generation, GenerationBand::Current);

            let direct = claim_by_id[rr.claim_ids[0].as_str()];
            let restored = claim_by_id[rr.claim_ids[1].as_str()];
            assert_eq!(direct.basis, ClaimBasis::DirectDescent);
            assert_eq!(direct.standing, ClaimStanding::Strong);
            assert!(direct.evidence_record_ids.is_empty());
            let direct_person = persons[direct.claimant_person_id.as_str()];
            assert_eq!(direct_person.house_id, ruling.id);
            assert_eq!(direct_person.generation, GenerationBand::Young);
            assert!(
                direct_person
                    .known_parent_ids
                    .iter()
                    .any(|p| p == &incumbent.id),
                "seed={seed} realm={} parent",
                rr.realm_id
            );
            assert_ne!(direct_person.id, incumbent.id);

            assert_eq!(restored.basis, ClaimBasis::RestoredLineRecord);
            assert_eq!(restored.standing, ClaimStanding::Contested);
            assert_eq!(restored.evidence_record_ids.len(), 1);
            let rec = rec_by_id[restored.evidence_record_ids[0].as_str()];
            assert_eq!(rec.realm_id, rr.realm_id);
            assert_eq!(rec.house_id, restored.claimant_house_id);
            assert_eq!(rec.house_id, cultural.id);
            let restored_person = persons[restored.claimant_person_id.as_str()];
            assert_eq!(restored_person.id, cultural.head_person_id);
            assert_eq!(restored_person.generation, GenerationBand::Current);
            assert_ne!(restored_person.id, incumbent.id);
            assert_ne!(restored_person.id, direct_person.id);

            let ri = realm_id[rr.realm_id.as_str()];
            let ruling_hi = house_id[ruling.id.as_str()];
            let cultural_hi = house_id[cultural.id.as_str()];
            assert_eq!(ruling_hi.culture_id, ri.majority_culture_id);
            assert_eq!(ruling_hi.religion_id, ri.majority_religion_id);
            assert_ne!(cultural_hi.culture_id, ri.majority_culture_id);
            assert_eq!(cultural_hi.religion_id, ri.majority_religion_id);
        }
    }
}

#[test]
fn claimants_unique_and_activity_layers() {
    for seed in SEEDS {
        let rw = generate_rights_world(seed).expect("rights");
        let active: BTreeMap<_, _> = rw
            .context_world
            .political
            .roster
            .active_actors
            .iter()
            .map(|a| (a.person_id.as_str(), a))
            .collect();
        let supporting: BTreeSet<_> = rw
            .context_world
            .political
            .roster
            .supporting_person_ids
            .iter()
            .map(|s| s.as_str())
            .collect();
        let mut claimants = BTreeSet::new();
        let mut direct_supporting = 0usize;
        let mut restored_active = 0usize;
        for claim in &rw.rights.claims {
            assert!(
                claimants.insert(claim.claimant_person_id.as_str()),
                "seed={seed} duplicate {}",
                claim.claimant_person_id
            );
            match claim.basis {
                ClaimBasis::DirectDescent => {
                    assert!(supporting.contains(claim.claimant_person_id.as_str()));
                    assert!(!active.contains_key(claim.claimant_person_id.as_str()));
                    direct_supporting += 1;
                }
                ClaimBasis::RestoredLineRecord => {
                    let actor = active[claim.claimant_person_id.as_str()];
                    assert_eq!(actor.primary_role, ActiveRole::HouseHead);
                    assert!(!supporting.contains(claim.claimant_person_id.as_str()));
                    restored_active += 1;
                }
            }
        }
        assert_eq!(claimants.len(), 12, "seed={seed}");
        assert_eq!(direct_supporting, 6, "seed={seed}");
        assert_eq!(restored_active, 6, "seed={seed}");
        for rr in &rw.rights.realms {
            assert!(!claimants.contains(rr.incumbent_person_id.as_str()));
        }
    }
}

#[test]
fn seed1_realm01_concrete_trace() {
    let rw = generate_rights_world(1).expect("rights");
    let rr = rw
        .rights
        .realms
        .iter()
        .find(|r| r.realm_id == "realm-01")
        .expect("realm-01");
    let houses = houses_sorted_for_realm(&rw, "realm-01");
    assert_eq!(houses.len(), 3);
    let h0 = houses[0];
    let h2 = houses[2];
    assert_eq!(h0.id, "house-01");

    let persons = person_by_id(&rw);
    let link = rw
        .context_world
        .political
        .dynastic
        .population
        .ruler_links
        .iter()
        .find(|l| {
            rw.context_world
                .political
                .dynastic
                .world
                .realms
                .iter()
                .any(|realm| realm.id == "realm-01" && realm.ruler_id == l.ruler_id)
        })
        .expect("ruler link");
    assert_eq!(rr.incumbent_person_id, link.person_id);
    assert_eq!(rr.incumbent_person_id, h0.head_person_id);
    let incumbent = persons[rr.incumbent_person_id.as_str()];
    assert_eq!(incumbent.realm_id, "realm-01");

    let direct_id = h0.member_ids.get(5).expect("member_ids[5]");
    let claim_by_id: BTreeMap<_, _> = rw
        .rights
        .claims
        .iter()
        .map(|c| (c.id.as_str(), c))
        .collect();
    assert_eq!(
        rr.claim_ids,
        vec!["claim-01".to_string(), "claim-02".to_string()]
    );
    let direct = claim_by_id["claim-01"];
    let restored = claim_by_id["claim-02"];
    assert_eq!(direct.claimant_person_id, *direct_id);
    let direct_person = persons[direct.claimant_person_id.as_str()];
    assert!(direct_person.known_parent_ids.contains(&incumbent.id));
    assert_eq!(direct_person.generation, GenerationBand::Young);
    assert_eq!(direct.basis, ClaimBasis::DirectDescent);
    assert_eq!(direct.standing, ClaimStanding::Strong);
    assert!(direct.evidence_record_ids.is_empty());

    let supporting: BTreeSet<_> = rw
        .context_world
        .political
        .roster
        .supporting_person_ids
        .iter()
        .map(|s| s.as_str())
        .collect();
    assert!(supporting.contains(direct_person.id.as_str()));

    assert_eq!(restored.claimant_person_id, h2.head_person_id);
    let restored_person = persons[restored.claimant_person_id.as_str()];
    assert_eq!(restored_person.generation, GenerationBand::Current);
    let ri = realm_identity_by_id(&rw)["realm-01"];
    let hi = house_identity_by_id(&rw)[h2.id.as_str()];
    assert_ne!(hi.culture_id, ri.majority_culture_id);
    assert_eq!(hi.religion_id, ri.majority_religion_id);
    let actor = rw
        .context_world
        .political
        .roster
        .active_actors
        .iter()
        .find(|a| a.person_id == restored_person.id)
        .expect("restored active");
    assert_eq!(actor.primary_role, ActiveRole::HouseHead);

    assert_eq!(
        restored.evidence_record_ids,
        vec!["right-record-01".to_string()]
    );
    let rec = rw
        .rights
        .evidence_records
        .iter()
        .find(|r| r.id == "right-record-01")
        .expect("evidence");
    assert_eq!(rec.realm_id, "realm-01");
    assert_eq!(rec.house_id, h2.id);

    assert_ne!(incumbent.id, direct_person.id);
    assert_ne!(incumbent.id, restored_person.id);
    assert_ne!(direct_person.id, restored_person.id);
}

#[test]
fn same_seed_structure_and_bytes_equal() {
    let a = generate_rights_world(1).expect("a");
    let b = generate_rights_world(1).expect("b");
    assert_eq!(a, b);
    assert_eq!(
        a.to_compact_json_bytes().unwrap(),
        b.to_compact_json_bytes().unwrap()
    );
    let c = generate_rights_world(2).expect("c");
    assert_ne!(
        a.to_compact_json_bytes().unwrap(),
        c.to_compact_json_bytes().unwrap()
    );
    assert_eq!(c.rights.realms.len(), 6);
    assert_eq!(c.rights.claims.len(), 12);
    assert_eq!(c.rights.evidence_records.len(), 6);
}

#[test]
fn context_and_population_rng_unchanged() {
    for seed in SEEDS {
        let context = generate_context_world(seed).expect("context");
        let dynastic = generate_dynastic_world(seed).expect("dynastic");
        let rw = generate_rights_world(seed).expect("rights");
        assert_eq!(rw.context_world, context, "seed={seed}");
        assert_eq!(
            rw.context_world.to_compact_json_bytes().unwrap(),
            context.to_compact_json_bytes().unwrap(),
            "seed={seed}"
        );
        assert_eq!(
            rw.context_world
                .political
                .dynastic
                .population
                .generation
                .rng_draws,
            dynastic.population.generation.rng_draws,
            "seed={seed}"
        );
        assert_eq!(
            rw.context_world
                .political
                .dynastic
                .world
                .generation
                .rng_draws,
            dynastic.world.generation.rng_draws,
            "seed={seed}"
        );
    }
}

#[test]
fn existing_schema_versions_unchanged() {
    assert_eq!(WORLD_SCHEMA_VERSION, 1);
    assert_eq!(DYNASTIC_WORLD_SCHEMA_VERSION, 1);
    assert_eq!(POLITICAL_WORLD_SCHEMA_VERSION, 1);
    assert_eq!(CONTEXT_WORLD_SCHEMA_VERSION, 1);
    assert_eq!(SAVE_SCHEMA_VERSION, 1);
    assert_eq!(RIGHTS_WORLD_SCHEMA_VERSION, 1);
}

#[test]
fn malformed_rights_fail_closed_no_panic() {
    let rw = generate_rights_world(1).expect("rights");
    let cw = &rw.context_world;

    let empty = InitialRights {
        realms: vec![],
        claims: vec![],
        evidence_records: vec![],
    };
    let err = validate_initial_rights(cw, &empty).unwrap_err();
    assert!(matches!(err, CoreError::InvalidRights(_)), "got {err:?}");

    let mut missing_claim = rw.rights.clone();
    missing_claim.claims.pop();
    let err = validate_initial_rights(cw, &missing_claim).unwrap_err();
    assert!(matches!(err, CoreError::InvalidRights(_)), "got {err:?}");

    let mut dup_claim = rw.rights.clone();
    if let Some(first) = dup_claim.claims.first().cloned()
        && let Some(last) = dup_claim.claims.last_mut()
    {
        last.id = first.id;
    }
    let err = validate_initial_rights(cw, &dup_claim).unwrap_err();
    assert!(matches!(err, CoreError::InvalidRights(_)), "got {err:?}");

    let mut missing_evidence = rw.rights.clone();
    missing_evidence.evidence_records.pop();
    let err = validate_initial_rights(cw, &missing_evidence).unwrap_err();
    assert!(matches!(err, CoreError::InvalidRights(_)), "got {err:?}");

    let mut dup_evidence = rw.rights.clone();
    if let Some(first) = dup_evidence.evidence_records.first().cloned()
        && let Some(last) = dup_evidence.evidence_records.last_mut()
    {
        last.id = first.id;
    }
    let err = validate_initial_rights(cw, &dup_evidence).unwrap_err();
    assert!(matches!(err, CoreError::InvalidRights(_)), "got {err:?}");

    let mut unknown_claimant = rw.rights.clone();
    unknown_claimant.claims[0].claimant_person_id = "person-does-not-exist".to_string();
    let err = validate_initial_rights(cw, &unknown_claimant).unwrap_err();
    assert!(matches!(err, CoreError::InvalidRights(_)), "got {err:?}");

    let mut unknown_house = rw.rights.clone();
    unknown_house.claims[0].claimant_house_id = "house-does-not-exist".to_string();
    let err = validate_initial_rights(cw, &unknown_house).unwrap_err();
    assert!(matches!(err, CoreError::InvalidRights(_)), "got {err:?}");

    let mut realm_mismatch = rw.rights.clone();
    realm_mismatch.claims[0].realm_id = "realm-02".to_string();
    let err = validate_initial_rights(cw, &realm_mismatch).unwrap_err();
    assert!(matches!(err, CoreError::InvalidRights(_)), "got {err:?}");

    let mut target_mismatch = rw.rights.clone();
    target_mismatch.claims[0].succession_target_key = "succession:realm-02".to_string();
    let err = validate_initial_rights(cw, &target_mismatch).unwrap_err();
    assert!(matches!(err, CoreError::InvalidRights(_)), "got {err:?}");

    let houses = houses_sorted_for_realm(&rw, "realm-01");
    let ruling = houses[0];
    let mut not_young = rw.rights.clone();
    not_young.claims[0].claimant_person_id = ruling.member_ids[4].clone();
    let err = validate_initial_rights(cw, &not_young).unwrap_err();
    assert!(matches!(err, CoreError::InvalidRights(_)), "got {err:?}");

    let mut not_child = rw.rights.clone();
    not_child.claims[0].claimant_person_id = ruling.member_ids[6].clone();
    let err = validate_initial_rights(cw, &not_child).unwrap_err();
    assert!(matches!(err, CoreError::InvalidRights(_)), "got {err:?}");

    let mut not_head = rw.rights.clone();
    not_head.claims[1].claimant_person_id = houses[2].member_ids[3].clone();
    let err = validate_initial_rights(cw, &not_head).unwrap_err();
    assert!(matches!(err, CoreError::InvalidRights(_)), "got {err:?}");

    let mut not_cultural = rw.rights.clone();
    not_cultural.claims[1].claimant_person_id = houses[1].head_person_id.clone();
    not_cultural.claims[1].claimant_house_id = houses[1].id.clone();
    not_cultural.evidence_records[0].house_id = houses[1].id.clone();
    let err = validate_initial_rights(cw, &not_cultural).unwrap_err();
    assert!(matches!(err, CoreError::InvalidRights(_)), "got {err:?}");

    let mut foreign_evidence = rw.rights.clone();
    foreign_evidence.claims[1].evidence_record_ids = vec!["right-record-02".to_string()];
    let err = validate_initial_rights(cw, &foreign_evidence).unwrap_err();
    assert!(matches!(err, CoreError::InvalidRights(_)), "got {err:?}");

    let mut incumbent_as_claimant = rw.rights.clone();
    incumbent_as_claimant.claims[0].claimant_person_id =
        rw.rights.realms[0].incumbent_person_id.clone();
    let err = validate_initial_rights(cw, &incumbent_as_claimant).unwrap_err();
    assert!(matches!(err, CoreError::InvalidRights(_)), "got {err:?}");

    let mut same_claimants = rw.rights.clone();
    same_claimants.claims[1].claimant_person_id =
        same_claimants.claims[0].claimant_person_id.clone();
    let err = validate_initial_rights(cw, &same_claimants).unwrap_err();
    assert!(matches!(err, CoreError::InvalidRights(_)), "got {err:?}");

    let mut roster_direct_active = rw.context_world.clone();
    let direct_id = rw.rights.claims[0].claimant_person_id.clone();
    roster_direct_active
        .political
        .roster
        .supporting_person_ids
        .retain(|id| id != &direct_id);
    roster_direct_active
        .political
        .roster
        .active_actors
        .push(epoch_core::ActiveActor {
            person_id: direct_id,
            realm_id: "realm-01".to_string(),
            primary_role: ActiveRole::RulingHouseCurrent,
            activation_reasons: vec![epoch_core::ActivationReason::RulingHouseCurrent],
        });
    roster_direct_active
        .political
        .roster
        .active_actors
        .sort_by(|a, b| a.person_id.cmp(&b.person_id));
    let err = validate_initial_rights(&roster_direct_active, &rw.rights).unwrap_err();
    assert!(matches!(err, CoreError::InvalidRights(_)), "got {err:?}");

    let mut roster_restored_supporting = rw.context_world.clone();
    let restored_id = rw.rights.claims[1].claimant_person_id.clone();
    roster_restored_supporting
        .political
        .roster
        .active_actors
        .retain(|a| a.person_id != restored_id);
    roster_restored_supporting
        .political
        .roster
        .supporting_person_ids
        .push(restored_id);
    roster_restored_supporting
        .political
        .roster
        .supporting_person_ids
        .sort();
    let err = validate_initial_rights(&roster_restored_supporting, &rw.rights).unwrap_err();
    assert!(matches!(err, CoreError::InvalidRights(_)), "got {err:?}");

    let ok = derive_initial_rights(cw).expect("derive");
    assert_eq!(ok.claims.len(), 12);
}

#[test]
fn corrupted_context_input_fail_closed_no_panic() {
    let mut cw = generate_context_world(1).expect("context");
    cw.political.dynastic.population.houses[0]
        .member_ids
        .truncate(1);
    let err = derive_initial_rights(&cw).expect_err("derive short members");
    assert!(matches!(err, CoreError::InvalidRights(_)), "got {err:?}");

    let mut cw_missing = generate_context_world(1).expect("context missing");
    cw_missing.political.dynastic.population.houses.pop();
    let err = derive_initial_rights(&cw_missing).expect_err("derive missing house");
    assert!(matches!(err, CoreError::InvalidRights(_)), "got {err:?}");
}

#[test]
fn arrays_explicitly_sorted() {
    let rw = generate_rights_world(1).expect("rights");
    let realm_ids: Vec<_> = rw
        .rights
        .realms
        .iter()
        .map(|r| r.realm_id.as_str())
        .collect();
    let mut sorted = realm_ids.clone();
    sorted.sort();
    assert_eq!(realm_ids, sorted);
    let claim_ids: Vec<_> = rw.rights.claims.iter().map(|c| c.id.as_str()).collect();
    let mut sorted_claims = claim_ids.clone();
    sorted_claims.sort();
    assert_eq!(claim_ids, sorted_claims);
    let rec_ids: Vec<_> = rw
        .rights
        .evidence_records
        .iter()
        .map(|r| r.id.as_str())
        .collect();
    let mut sorted_recs = rec_ids.clone();
    sorted_recs.sort();
    assert_eq!(rec_ids, sorted_recs);
    for rr in &rw.rights.realms {
        let mut ids = rr.claim_ids.clone();
        ids.sort();
        assert_eq!(rr.claim_ids, ids);
    }
}

// rights-check 1/2를 포함한 M0~M1.5 exact 회귀는
// common::CLI_EXACT_REGRESSION이 한 곳에서 담당한다.

#[test]
fn cli_rights_1_succeeds() {
    common::assert_cli_json_eq(
        &["rights", "1"],
        &generate_rights_world(1).expect("rights 1"),
    );
}

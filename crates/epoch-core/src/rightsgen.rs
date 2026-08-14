// M1.5 초기 계승 권리 생성·검증 — RNG 없이 기존 stable ID와 가계에서 파생

use crate::context::ContextWorld;
use crate::contextgen::validate_initial_context;
use crate::error::CoreError;
use crate::political::ActiveRole;
use crate::politicalgen::validate_political_roster;
use crate::population::{GenerationBand, HOUSES_PER_REALM};
use crate::populationgen::validate_population;
use crate::rights::{
    ClaimBasis, ClaimStanding, InitialRights, REALM_RIGHTS_COUNT, RIGHT_EVIDENCE_COUNT,
    RIGHTS_WORLD_SCHEMA_VERSION, RealmRights, RightEvidenceKind, RightEvidenceRecord, RightsWorld,
    SUCCESSION_CLAIM_COUNT, SuccessionClaim, succession_target_key,
};
use crate::world::WORLD_REALM_COUNT;
use crate::worldgen::validate_world;
use std::collections::{BTreeMap, BTreeSet};

fn invalid_rights(msg: impl Into<String>) -> CoreError {
    CoreError::InvalidRights(msg.into())
}

fn map_layer_error(err: CoreError) -> CoreError {
    match err {
        CoreError::InvalidWorld(msg) => invalid_rights(format!("world: {msg}")),
        CoreError::InvalidPopulation(msg) => invalid_rights(format!("population: {msg}")),
        CoreError::InvalidPolitical(msg) => invalid_rights(format!("political: {msg}")),
        CoreError::InvalidContext(msg) => invalid_rights(format!("context: {msg}")),
        other => other,
    }
}

fn validate_lower_layers(context_world: &ContextWorld) -> Result<(), CoreError> {
    validate_world(&context_world.political.dynastic.world).map_err(map_layer_error)?;
    validate_population(
        &context_world.political.dynastic.world,
        &context_world.political.dynastic.population,
    )
    .map_err(map_layer_error)?;
    validate_political_roster(
        &context_world.political.dynastic,
        &context_world.political.roster,
    )
    .map_err(map_layer_error)?;
    validate_initial_context(&context_world.political, &context_world.context)
        .map_err(map_layer_error)?;
    Ok(())
}

/// realm별 house를 ID 오름차순으로 묶는다 (local[0]=ruling, local[2]=cultural minority).
fn houses_by_realm_sorted(
    context_world: &ContextWorld,
) -> Result<BTreeMap<String, Vec<&crate::population::House>>, CoreError> {
    let mut map: BTreeMap<String, Vec<&crate::population::House>> = BTreeMap::new();
    for house in &context_world.political.dynastic.population.houses {
        map.entry(house.realm_id.clone()).or_default().push(house);
    }
    for (realm_id, houses) in map.iter_mut() {
        houses.sort_by(|a, b| a.id.cmp(&b.id));
        if houses.len() != HOUSES_PER_REALM {
            return Err(invalid_rights(format!(
                "realm {realm_id} houses {} != {HOUSES_PER_REALM}",
                houses.len()
            )));
        }
    }
    if map.len() != WORLD_REALM_COUNT {
        return Err(invalid_rights(format!(
            "realm house groups {} != {WORLD_REALM_COUNT}",
            map.len()
        )));
    }
    Ok(map)
}

fn sorted_realms(context_world: &ContextWorld) -> Result<Vec<&crate::world::Realm>, CoreError> {
    let mut realms: Vec<&crate::world::Realm> = context_world
        .political
        .dynastic
        .world
        .realms
        .iter()
        .collect();
    realms.sort_by(|a, b| a.id.cmp(&b.id));
    if realms.len() != WORLD_REALM_COUNT {
        return Err(invalid_rights(format!(
            "realm count {} != {WORLD_REALM_COUNT}",
            realms.len()
        )));
    }
    Ok(realms)
}

/// ContextWorld에서 초기 계승 권리를 파생한다. RNG를 사용하지 않는다.
pub fn derive_initial_rights(context_world: &ContextWorld) -> Result<InitialRights, CoreError> {
    validate_lower_layers(context_world)?;

    let pop = &context_world.political.dynastic.population;
    let person_by_id: BTreeMap<&str, &crate::population::Person> =
        pop.persons.iter().map(|p| (p.id.as_str(), p)).collect();
    let ruler_by_id: BTreeMap<&str, &crate::world::Ruler> = context_world
        .political
        .dynastic
        .world
        .rulers
        .iter()
        .map(|r| (r.id.as_str(), r))
        .collect();
    let link_by_ruler: BTreeMap<&str, &crate::population::RulerPersonLink> = pop
        .ruler_links
        .iter()
        .map(|l| (l.ruler_id.as_str(), l))
        .collect();

    let realms = sorted_realms(context_world)?;
    let houses_by_realm = houses_by_realm_sorted(context_world)?;

    let mut realm_rights: Vec<RealmRights> = Vec::with_capacity(REALM_RIGHTS_COUNT);
    let mut claims: Vec<SuccessionClaim> = Vec::with_capacity(SUCCESSION_CLAIM_COUNT);
    let mut evidence_records: Vec<RightEvidenceRecord> = Vec::with_capacity(RIGHT_EVIDENCE_COUNT);

    for (realm_idx, realm) in realms.iter().enumerate() {
        let houses = houses_by_realm
            .get(realm.id.as_str())
            .ok_or_else(|| invalid_rights(format!("missing houses for realm {}", realm.id)))?;
        let ruling = houses
            .first()
            .ok_or_else(|| invalid_rights(format!("realm {} missing ruling house", realm.id)))?;
        let cultural = houses.get(2).ok_or_else(|| {
            invalid_rights(format!(
                "realm {} missing cultural-minority house",
                realm.id
            ))
        })?;

        let ruler = ruler_by_id.get(realm.ruler_id.as_str()).ok_or_else(|| {
            invalid_rights(format!(
                "realm {} ruler {} missing",
                realm.id, realm.ruler_id
            ))
        })?;
        if ruler.realm_id != realm.id {
            return Err(invalid_rights(format!(
                "ruler {} realm {} != {}",
                ruler.id, ruler.realm_id, realm.id
            )));
        }
        let link = link_by_ruler.get(realm.ruler_id.as_str()).ok_or_else(|| {
            invalid_rights(format!(
                "realm {} ruler {} has no person link",
                realm.id, realm.ruler_id
            ))
        })?;
        let incumbent = person_by_id.get(link.person_id.as_str()).ok_or_else(|| {
            invalid_rights(format!(
                "incumbent person {} missing for realm {}",
                link.person_id, realm.id
            ))
        })?;
        if incumbent.id != ruling.head_person_id {
            return Err(invalid_rights(format!(
                "incumbent {} != ruling house {} head {}",
                incumbent.id, ruling.id, ruling.head_person_id
            )));
        }

        let direct_id = ruling.member_ids.get(5).ok_or_else(|| {
            invalid_rights(format!("ruling house {} missing member_ids[5]", ruling.id))
        })?;
        let direct = person_by_id.get(direct_id.as_str()).ok_or_else(|| {
            invalid_rights(format!(
                "direct claimant {} missing for realm {}",
                direct_id, realm.id
            ))
        })?;
        let restored = person_by_id
            .get(cultural.head_person_id.as_str())
            .ok_or_else(|| {
                invalid_rights(format!(
                    "restored claimant {} missing for realm {}",
                    cultural.head_person_id, realm.id
                ))
            })?;

        let claim_direct_id = format!("claim-{:02}", realm_idx * 2 + 1);
        let claim_restored_id = format!("claim-{:02}", realm_idx * 2 + 2);
        let evidence_id = format!("right-record-{:02}", realm_idx + 1);
        let target = succession_target_key(&realm.id);

        let mut claim_ids = vec![claim_direct_id.clone(), claim_restored_id.clone()];
        claim_ids.sort();

        realm_rights.push(RealmRights {
            realm_id: realm.id.clone(),
            succession_target_key: target.clone(),
            incumbent_person_id: incumbent.id.clone(),
            claim_ids,
        });

        claims.push(SuccessionClaim {
            id: claim_direct_id,
            realm_id: realm.id.clone(),
            succession_target_key: target.clone(),
            claimant_person_id: direct.id.clone(),
            claimant_house_id: ruling.id.clone(),
            basis: ClaimBasis::DirectDescent,
            standing: ClaimStanding::Strong,
            evidence_record_ids: vec![],
        });

        let mut evidence_record_ids = vec![evidence_id.clone()];
        evidence_record_ids.sort();
        claims.push(SuccessionClaim {
            id: claim_restored_id,
            realm_id: realm.id.clone(),
            succession_target_key: target,
            claimant_person_id: restored.id.clone(),
            claimant_house_id: cultural.id.clone(),
            basis: ClaimBasis::RestoredLineRecord,
            standing: ClaimStanding::Contested,
            evidence_record_ids,
        });

        evidence_records.push(RightEvidenceRecord {
            id: evidence_id,
            realm_id: realm.id.clone(),
            house_id: cultural.id.clone(),
            kind: RightEvidenceKind::RestoredLineage,
        });
    }

    realm_rights.sort_by(|a, b| a.realm_id.cmp(&b.realm_id));
    claims.sort_by(|a, b| a.id.cmp(&b.id));
    evidence_records.sort_by(|a, b| a.id.cmp(&b.id));

    let rights = InitialRights {
        realms: realm_rights,
        claims,
        evidence_records,
    };
    validate_initial_rights(context_world, &rights)?;
    Ok(rights)
}

/// seed에서 RightsWorld를 생성한다.
pub fn generate_rights_world(seed: u64) -> Result<RightsWorld, CoreError> {
    let context_world = crate::contextgen::generate_context_world(seed)?;
    validate_lower_layers(&context_world)?;
    let rights = derive_initial_rights(&context_world)?;
    validate_initial_rights(&context_world, &rights)?;
    Ok(RightsWorld {
        schema_version: RIGHTS_WORLD_SCHEMA_VERSION,
        seed,
        context_world,
        rights,
    })
}

/// 초기 계승 권리 불변식을 검사한다. 실패 시 fail closed.
pub fn validate_initial_rights(
    context_world: &ContextWorld,
    rights: &InitialRights,
) -> Result<(), CoreError> {
    validate_lower_layers(context_world)?;

    let pop = &context_world.political.dynastic.population;
    let person_by_id: BTreeMap<&str, &crate::population::Person> =
        pop.persons.iter().map(|p| (p.id.as_str(), p)).collect();
    let house_by_id: BTreeMap<&str, &crate::population::House> =
        pop.houses.iter().map(|h| (h.id.as_str(), h)).collect();
    let realm_ids: BTreeSet<&str> = context_world
        .political
        .dynastic
        .world
        .realms
        .iter()
        .map(|r| r.id.as_str())
        .collect();
    let ruler_by_id: BTreeMap<&str, &crate::world::Ruler> = context_world
        .political
        .dynastic
        .world
        .rulers
        .iter()
        .map(|r| (r.id.as_str(), r))
        .collect();
    let realm_by_id: BTreeMap<&str, &crate::world::Realm> = context_world
        .political
        .dynastic
        .world
        .realms
        .iter()
        .map(|r| (r.id.as_str(), r))
        .collect();
    let link_by_ruler: BTreeMap<&str, &crate::population::RulerPersonLink> = pop
        .ruler_links
        .iter()
        .map(|l| (l.ruler_id.as_str(), l))
        .collect();
    let realm_identity_by_id: BTreeMap<&str, &crate::context::RealmIdentity> = context_world
        .context
        .realm_identities
        .iter()
        .map(|r| (r.realm_id.as_str(), r))
        .collect();
    let house_identity_by_id: BTreeMap<&str, &crate::context::HouseIdentity> = context_world
        .context
        .house_identities
        .iter()
        .map(|h| (h.house_id.as_str(), h))
        .collect();
    let active_by_person: BTreeMap<&str, &crate::political::ActiveActor> = context_world
        .political
        .roster
        .active_actors
        .iter()
        .map(|a| (a.person_id.as_str(), a))
        .collect();
    let supporting: BTreeSet<&str> = context_world
        .political
        .roster
        .supporting_person_ids
        .iter()
        .map(|s| s.as_str())
        .collect();

    if rights.realms.len() != REALM_RIGHTS_COUNT {
        return Err(invalid_rights(format!(
            "realm rights {} != {REALM_RIGHTS_COUNT}",
            rights.realms.len()
        )));
    }
    if rights.claims.len() != SUCCESSION_CLAIM_COUNT {
        return Err(invalid_rights(format!(
            "claims {} != {SUCCESSION_CLAIM_COUNT}",
            rights.claims.len()
        )));
    }
    if rights.evidence_records.len() != RIGHT_EVIDENCE_COUNT {
        return Err(invalid_rights(format!(
            "evidence records {} != {RIGHT_EVIDENCE_COUNT}",
            rights.evidence_records.len()
        )));
    }

    for window in rights.realms.windows(2) {
        let a = window
            .first()
            .ok_or_else(|| invalid_rights("realm window"))?;
        let b = window
            .get(1)
            .ok_or_else(|| invalid_rights("realm window"))?;
        if a.realm_id >= b.realm_id {
            return Err(invalid_rights("realm rights not sorted by realm_id"));
        }
    }
    for window in rights.claims.windows(2) {
        let a = window
            .first()
            .ok_or_else(|| invalid_rights("claim window"))?;
        let b = window
            .get(1)
            .ok_or_else(|| invalid_rights("claim window"))?;
        if a.id >= b.id {
            return Err(invalid_rights("claims not sorted by id"));
        }
    }
    for window in rights.evidence_records.windows(2) {
        let a = window
            .first()
            .ok_or_else(|| invalid_rights("evidence window"))?;
        let b = window
            .get(1)
            .ok_or_else(|| invalid_rights("evidence window"))?;
        if a.id >= b.id {
            return Err(invalid_rights("evidence records not sorted by id"));
        }
    }

    let mut realm_right_by_id: BTreeMap<&str, &RealmRights> = BTreeMap::new();
    for rr in &rights.realms {
        if !realm_ids.contains(rr.realm_id.as_str()) {
            return Err(invalid_rights(format!(
                "realm rights {} unknown realm",
                rr.realm_id
            )));
        }
        if realm_right_by_id.insert(rr.realm_id.as_str(), rr).is_some() {
            return Err(invalid_rights(format!(
                "duplicate realm rights {}",
                rr.realm_id
            )));
        }
        if rr.succession_target_key != succession_target_key(&rr.realm_id) {
            return Err(invalid_rights(format!(
                "realm {} succession target {} mismatch",
                rr.realm_id, rr.succession_target_key
            )));
        }
        if rr.claim_ids.len() != 2 {
            return Err(invalid_rights(format!(
                "realm {} claim_ids {} != 2",
                rr.realm_id,
                rr.claim_ids.len()
            )));
        }
        for window in rr.claim_ids.windows(2) {
            let a = window
                .first()
                .ok_or_else(|| invalid_rights("claim_ids window"))?;
            let b = window
                .get(1)
                .ok_or_else(|| invalid_rights("claim_ids window"))?;
            if a >= b {
                return Err(invalid_rights(format!(
                    "realm {} claim_ids not sorted",
                    rr.realm_id
                )));
            }
        }
    }
    if realm_right_by_id.len() != WORLD_REALM_COUNT {
        return Err(invalid_rights("realm rights coverage incomplete"));
    }
    for rid in &realm_ids {
        if !realm_right_by_id.contains_key(rid) {
            return Err(invalid_rights(format!("missing realm rights for {rid}")));
        }
    }

    let mut claim_by_id: BTreeMap<&str, &SuccessionClaim> = BTreeMap::new();
    for claim in &rights.claims {
        if claim_by_id.insert(claim.id.as_str(), claim).is_some() {
            return Err(invalid_rights(format!("duplicate claim {}", claim.id)));
        }
        if !realm_ids.contains(claim.realm_id.as_str()) {
            return Err(invalid_rights(format!(
                "claim {} unknown realm {}",
                claim.id, claim.realm_id
            )));
        }
        if claim.succession_target_key != succession_target_key(&claim.realm_id) {
            return Err(invalid_rights(format!(
                "claim {} succession target mismatch",
                claim.id
            )));
        }
        for window in claim.evidence_record_ids.windows(2) {
            let a = window
                .first()
                .ok_or_else(|| invalid_rights("evidence_record_ids window"))?;
            let b = window
                .get(1)
                .ok_or_else(|| invalid_rights("evidence_record_ids window"))?;
            if a >= b {
                return Err(invalid_rights(format!(
                    "claim {} evidence_record_ids not sorted",
                    claim.id
                )));
            }
        }
    }

    let mut evidence_by_id: BTreeMap<&str, &RightEvidenceRecord> = BTreeMap::new();
    for rec in &rights.evidence_records {
        if evidence_by_id.insert(rec.id.as_str(), rec).is_some() {
            return Err(invalid_rights(format!("duplicate evidence {}", rec.id)));
        }
        if !realm_ids.contains(rec.realm_id.as_str()) {
            return Err(invalid_rights(format!(
                "evidence {} unknown realm {}",
                rec.id, rec.realm_id
            )));
        }
        if !house_by_id.contains_key(rec.house_id.as_str()) {
            return Err(invalid_rights(format!(
                "evidence {} unknown house {}",
                rec.id, rec.house_id
            )));
        }
        if rec.kind != RightEvidenceKind::RestoredLineage {
            return Err(invalid_rights(format!(
                "evidence {} unexpected kind",
                rec.id
            )));
        }
    }

    let mut referenced_claims: BTreeSet<&str> = BTreeSet::new();
    let mut referenced_evidence: BTreeSet<&str> = BTreeSet::new();
    let mut claimant_ids: BTreeSet<&str> = BTreeSet::new();
    let mut direct_count = 0usize;
    let mut restored_count = 0usize;
    let mut strong_count = 0usize;
    let mut contested_count = 0usize;
    let mut direct_supporting = 0usize;
    let mut restored_active = 0usize;

    let houses_by_realm = houses_by_realm_sorted(context_world)?;
    let realms = sorted_realms(context_world)?;

    for (realm_idx, realm) in realms.iter().enumerate() {
        let rr = realm_right_by_id
            .get(realm.id.as_str())
            .ok_or_else(|| invalid_rights(format!("missing realm rights {}", realm.id)))?;
        let expected_direct = format!("claim-{:02}", realm_idx * 2 + 1);
        let expected_restored = format!("claim-{:02}", realm_idx * 2 + 2);
        let expected_evidence = format!("right-record-{:02}", realm_idx + 1);
        if rr.claim_ids.first().map(String::as_str) != Some(expected_direct.as_str())
            || rr.claim_ids.get(1).map(String::as_str) != Some(expected_restored.as_str())
        {
            return Err(invalid_rights(format!(
                "realm {} claim_ids {:?} != [{expected_direct}, {expected_restored}]",
                realm.id, rr.claim_ids
            )));
        }

        let houses = houses_by_realm
            .get(realm.id.as_str())
            .ok_or_else(|| invalid_rights(format!("missing houses for realm {}", realm.id)))?;
        let ruling = houses
            .first()
            .ok_or_else(|| invalid_rights(format!("realm {} missing ruling house", realm.id)))?;
        let cultural = houses.get(2).ok_or_else(|| {
            invalid_rights(format!(
                "realm {} missing cultural-minority house",
                realm.id
            ))
        })?;

        let realm_identity = realm_identity_by_id
            .get(realm.id.as_str())
            .ok_or_else(|| invalid_rights(format!("missing realm identity {}", realm.id)))?;
        let ruling_identity = house_identity_by_id
            .get(ruling.id.as_str())
            .ok_or_else(|| invalid_rights(format!("missing house identity {}", ruling.id)))?;
        let cultural_identity = house_identity_by_id
            .get(cultural.id.as_str())
            .ok_or_else(|| invalid_rights(format!("missing house identity {}", cultural.id)))?;
        if ruling_identity.culture_id != realm_identity.majority_culture_id
            || ruling_identity.religion_id != realm_identity.majority_religion_id
        {
            return Err(invalid_rights(format!(
                "ruling house {} is not realm {} majority identity",
                ruling.id, realm.id
            )));
        }
        if cultural_identity.culture_id == realm_identity.majority_culture_id
            || cultural_identity.religion_id != realm_identity.majority_religion_id
        {
            return Err(invalid_rights(format!(
                "house {} is not cultural-minority of realm {}",
                cultural.id, realm.id
            )));
        }

        let world_realm = realm_by_id
            .get(realm.id.as_str())
            .ok_or_else(|| invalid_rights(format!("missing world realm {}", realm.id)))?;
        let ruler = ruler_by_id
            .get(world_realm.ruler_id.as_str())
            .ok_or_else(|| {
                invalid_rights(format!(
                    "realm {} ruler {} missing",
                    realm.id, world_realm.ruler_id
                ))
            })?;
        if ruler.realm_id != realm.id {
            return Err(invalid_rights(format!(
                "ruler {} realm mismatch for {}",
                ruler.id, realm.id
            )));
        }
        let link = link_by_ruler
            .get(world_realm.ruler_id.as_str())
            .ok_or_else(|| {
                invalid_rights(format!(
                    "realm {} ruler {} has no person link",
                    realm.id, world_realm.ruler_id
                ))
            })?;
        if rr.incumbent_person_id != link.person_id {
            return Err(invalid_rights(format!(
                "realm {} incumbent {} != ruler link {}",
                realm.id, rr.incumbent_person_id, link.person_id
            )));
        }
        let incumbent = person_by_id
            .get(rr.incumbent_person_id.as_str())
            .ok_or_else(|| {
                invalid_rights(format!(
                    "incumbent {} missing for realm {}",
                    rr.incumbent_person_id, realm.id
                ))
            })?;
        if incumbent.realm_id != realm.id {
            return Err(invalid_rights(format!(
                "incumbent {} realm {} != {}",
                incumbent.id, incumbent.realm_id, realm.id
            )));
        }
        if incumbent.generation != GenerationBand::Current {
            return Err(invalid_rights(format!(
                "incumbent {} is not Current",
                incumbent.id
            )));
        }
        if incumbent.id != ruling.head_person_id {
            return Err(invalid_rights(format!(
                "incumbent {} != ruling house head {}",
                incumbent.id, ruling.head_person_id
            )));
        }

        let direct_claim = claim_by_id
            .get(expected_direct.as_str())
            .ok_or_else(|| invalid_rights(format!("missing direct claim {expected_direct}")))?;
        let restored_claim = claim_by_id
            .get(expected_restored.as_str())
            .ok_or_else(|| invalid_rights(format!("missing restored claim {expected_restored}")))?;
        if !referenced_claims.insert(direct_claim.id.as_str())
            || !referenced_claims.insert(restored_claim.id.as_str())
        {
            return Err(invalid_rights(format!(
                "realm {} claim referenced twice",
                realm.id
            )));
        }
        if direct_claim.realm_id != realm.id || restored_claim.realm_id != realm.id {
            return Err(invalid_rights(format!(
                "realm {} claim realm mismatch",
                realm.id
            )));
        }
        if direct_claim.succession_target_key != rr.succession_target_key
            || restored_claim.succession_target_key != rr.succession_target_key
        {
            return Err(invalid_rights(format!(
                "realm {} claim succession target mismatch",
                realm.id
            )));
        }

        if direct_claim.basis != ClaimBasis::DirectDescent
            || direct_claim.standing != ClaimStanding::Strong
        {
            return Err(invalid_rights(format!(
                "claim {} must be DirectDescent/Strong",
                direct_claim.id
            )));
        }
        if !direct_claim.evidence_record_ids.is_empty() {
            return Err(invalid_rights(format!(
                "direct claim {} must have empty evidence",
                direct_claim.id
            )));
        }
        let direct = person_by_id
            .get(direct_claim.claimant_person_id.as_str())
            .ok_or_else(|| {
                invalid_rights(format!(
                    "direct claimant {} missing",
                    direct_claim.claimant_person_id
                ))
            })?;
        if !house_by_id.contains_key(direct_claim.claimant_house_id.as_str()) {
            return Err(invalid_rights(format!(
                "claim {} unknown house {}",
                direct_claim.id, direct_claim.claimant_house_id
            )));
        }
        if direct.house_id != ruling.id
            || direct_claim.claimant_house_id != ruling.id
            || !ruling.member_ids.contains(&direct.id)
        {
            return Err(invalid_rights(format!(
                "direct claimant {} is not a ruling house member",
                direct.id
            )));
        }
        if direct.generation != GenerationBand::Young {
            return Err(invalid_rights(format!(
                "direct claimant {} is not Young",
                direct.id
            )));
        }
        if !direct
            .known_parent_ids
            .iter()
            .any(|pid| pid == &incumbent.id)
        {
            return Err(invalid_rights(format!(
                "direct claimant {} is not a known child of incumbent {}",
                direct.id, incumbent.id
            )));
        }
        if direct.realm_id != realm.id {
            return Err(invalid_rights(format!(
                "direct claimant {} realm mismatch",
                direct.id
            )));
        }
        if direct.id == incumbent.id {
            return Err(invalid_rights(format!(
                "incumbent {} cannot be a claimant",
                incumbent.id
            )));
        }
        if active_by_person.contains_key(direct.id.as_str()) {
            return Err(invalid_rights(format!(
                "direct claimant {} must be Supporting, not Active",
                direct.id
            )));
        }
        if !supporting.contains(direct.id.as_str()) {
            return Err(invalid_rights(format!(
                "direct claimant {} is not Supporting",
                direct.id
            )));
        }
        if !claimant_ids.insert(direct.id.as_str()) {
            return Err(invalid_rights(format!("duplicate claimant {}", direct.id)));
        }
        direct_count += 1;
        strong_count += 1;
        direct_supporting += 1;

        if restored_claim.basis != ClaimBasis::RestoredLineRecord
            || restored_claim.standing != ClaimStanding::Contested
        {
            return Err(invalid_rights(format!(
                "claim {} must be RestoredLineRecord/Contested",
                restored_claim.id
            )));
        }
        if restored_claim.evidence_record_ids.len() != 1 {
            return Err(invalid_rights(format!(
                "restored claim {} evidence count {} != 1",
                restored_claim.id,
                restored_claim.evidence_record_ids.len()
            )));
        }
        let evidence_id = restored_claim.evidence_record_ids.first().ok_or_else(|| {
            invalid_rights(format!(
                "restored claim {} missing evidence id",
                restored_claim.id
            ))
        })?;
        if evidence_id != &expected_evidence {
            return Err(invalid_rights(format!(
                "restored claim {} evidence {evidence_id} != {expected_evidence}",
                restored_claim.id
            )));
        }
        if !referenced_evidence.insert(evidence_id.as_str()) {
            return Err(invalid_rights(format!(
                "evidence {evidence_id} referenced twice"
            )));
        }
        let rec = evidence_by_id
            .get(evidence_id.as_str())
            .ok_or_else(|| invalid_rights(format!("evidence {evidence_id} missing")))?;
        if rec.realm_id != realm.id || rec.realm_id != restored_claim.realm_id {
            return Err(invalid_rights(format!(
                "evidence {evidence_id} realm mismatch"
            )));
        }
        if rec.house_id != restored_claim.claimant_house_id {
            return Err(invalid_rights(format!(
                "evidence {evidence_id} house {} != claimant house {}",
                rec.house_id, restored_claim.claimant_house_id
            )));
        }
        let restored = person_by_id
            .get(restored_claim.claimant_person_id.as_str())
            .ok_or_else(|| {
                invalid_rights(format!(
                    "restored claimant {} missing",
                    restored_claim.claimant_person_id
                ))
            })?;
        if !house_by_id.contains_key(restored_claim.claimant_house_id.as_str()) {
            return Err(invalid_rights(format!(
                "claim {} unknown house {}",
                restored_claim.id, restored_claim.claimant_house_id
            )));
        }
        if restored.house_id != cultural.id
            || restored_claim.claimant_house_id != cultural.id
            || rec.house_id != cultural.id
        {
            return Err(invalid_rights(format!(
                "restored claimant {} is not cultural-minority house {}",
                restored.id, cultural.id
            )));
        }
        if cultural.id == ruling.id {
            return Err(invalid_rights(format!(
                "restored house {} is the ruling house",
                cultural.id
            )));
        }
        if restored.id != cultural.head_person_id {
            return Err(invalid_rights(format!(
                "restored claimant {} is not house head {}",
                restored.id, cultural.head_person_id
            )));
        }
        if restored.generation != GenerationBand::Current {
            return Err(invalid_rights(format!(
                "restored claimant {} is not Current",
                restored.id
            )));
        }
        if restored.realm_id != realm.id {
            return Err(invalid_rights(format!(
                "restored claimant {} realm mismatch",
                restored.id
            )));
        }
        if restored.id == incumbent.id || restored.id == direct.id {
            return Err(invalid_rights(format!(
                "restored claimant {} collides with incumbent or direct claimant",
                restored.id
            )));
        }
        let actor = active_by_person.get(restored.id.as_str()).ok_or_else(|| {
            invalid_rights(format!(
                "restored claimant {} must be Active, not Supporting",
                restored.id
            ))
        })?;
        if actor.primary_role != ActiveRole::HouseHead {
            return Err(invalid_rights(format!(
                "restored claimant {} primary role {:?} != HouseHead",
                restored.id, actor.primary_role
            )));
        }
        if supporting.contains(restored.id.as_str()) {
            return Err(invalid_rights(format!(
                "restored claimant {} is Supporting",
                restored.id
            )));
        }
        if !claimant_ids.insert(restored.id.as_str()) {
            return Err(invalid_rights(format!(
                "duplicate claimant {}",
                restored.id
            )));
        }
        restored_count += 1;
        contested_count += 1;
        restored_active += 1;
    }

    if referenced_claims.len() != SUCCESSION_CLAIM_COUNT {
        return Err(invalid_rights("claim coverage incomplete"));
    }
    if referenced_evidence.len() != RIGHT_EVIDENCE_COUNT {
        return Err(invalid_rights("evidence coverage incomplete"));
    }
    if claimant_ids.len() != SUCCESSION_CLAIM_COUNT {
        return Err(invalid_rights(format!(
            "unique claimants {} != {SUCCESSION_CLAIM_COUNT}",
            claimant_ids.len()
        )));
    }
    for rr in &rights.realms {
        if claimant_ids.contains(rr.incumbent_person_id.as_str()) {
            return Err(invalid_rights(format!(
                "incumbent {} is also a claimant",
                rr.incumbent_person_id
            )));
        }
    }
    if direct_count != REALM_RIGHTS_COUNT
        || restored_count != REALM_RIGHTS_COUNT
        || strong_count != REALM_RIGHTS_COUNT
        || contested_count != REALM_RIGHTS_COUNT
        || direct_supporting != REALM_RIGHTS_COUNT
        || restored_active != REALM_RIGHTS_COUNT
    {
        return Err(invalid_rights(format!(
            "basis/standing/activity counts direct={direct_count} restored={restored_count} strong={strong_count} contested={contested_count} supporting={direct_supporting} active={restored_active}"
        )));
    }

    Ok(())
}

// M1.2 인구·가계 골격 CLI·결정론 통합 테스트

use epoch_core::{
    GenerationBand, HOUSE_COUNT, PERSON_COUNT, generate_dynastic_world, generate_world,
    validate_population, validate_world,
};
use std::collections::{BTreeMap, BTreeSet};

mod common;

#[test]
fn exact_counts_and_generations() {
    for seed in [0u64, 1, 2, 42, u64::MAX] {
        let d = generate_dynastic_world(seed).expect("dynastic");
        assert_eq!(d.population.houses.len(), HOUSE_COUNT);
        assert_eq!(d.population.persons.len(), PERSON_COUNT);
        assert_eq!(d.population.ruler_links.len(), 6);

        let mut elder = 0usize;
        let mut current = 0usize;
        let mut young = 0usize;
        for p in &d.population.persons {
            match p.generation {
                GenerationBand::Elder => elder += 1,
                GenerationBand::Current => current += 1,
                GenerationBand::Young => young += 1,
            }
        }
        assert_eq!(elder, 36);
        assert_eq!(current, 54);
        assert_eq!(young, 54);
    }
}

#[test]
fn realm_distribution_three_houses_twenty_four_persons() {
    let d = generate_dynastic_world(1).expect("dynastic");
    for realm in &d.world.realms {
        let houses: Vec<_> = d
            .population
            .houses
            .iter()
            .filter(|h| h.realm_id == realm.id)
            .collect();
        assert_eq!(houses.len(), 3, "realm {}", realm.id);
        let persons: Vec<_> = d
            .population
            .persons
            .iter()
            .filter(|p| p.realm_id == realm.id)
            .collect();
        assert_eq!(persons.len(), 24, "realm {}", realm.id);
    }
}

#[test]
fn house_generation_composition_and_head_current() {
    let d = generate_dynastic_world(42).expect("dynastic");
    let by_id: BTreeMap<_, _> = d
        .population
        .persons
        .iter()
        .map(|p| (p.id.as_str(), p))
        .collect();
    for house in &d.population.houses {
        assert_eq!(house.member_ids.len(), 8);
        // 고정 슬롯: 0,1 Elder / 2,3,4 Current / 5,6,7 Young; head == member 2
        assert_eq!(house.head_person_id, house.member_ids[2]);
        let mut e = 0;
        let mut c = 0;
        let mut y = 0;
        for (m, mid) in house.member_ids.iter().enumerate() {
            let band = by_id[mid.as_str()].generation;
            match band {
                GenerationBand::Elder => e += 1,
                GenerationBand::Current => c += 1,
                GenerationBand::Young => y += 1,
            }
            let expected = match m {
                0 | 1 => GenerationBand::Elder,
                2..=4 => GenerationBand::Current,
                5..=7 => GenerationBand::Young,
                _ => unreachable!(),
            };
            assert_eq!(band, expected, "house {} member {m}", house.id);
        }
        assert_eq!((e, c, y), (2, 3, 3), "house {}", house.id);
        let head = by_id[house.head_person_id.as_str()];
        assert_eq!(head.generation, GenerationBand::Current);
        assert_eq!(head.home_territory_id, house.seat_territory_id);
        assert_eq!(head.realm_id, house.realm_id);
    }
}

#[test]
fn house_ids_stable_under_realm_vector_reorder() {
    let mut world = generate_world(1).expect("world");
    world.realms.reverse();
    validate_world(&world).expect("world ok");
    let pop = epoch_core::generate_population(&world).expect("pop");
    let mut sorted_realm_ids: Vec<_> = world.realms.iter().map(|r| r.id.as_str()).collect();
    sorted_realm_ids.sort();
    for (realm_index, realm_id) in sorted_realm_ids.iter().enumerate() {
        let mut houses: Vec<_> = pop
            .houses
            .iter()
            .filter(|h| h.realm_id == *realm_id)
            .collect();
        houses.sort_by(|a, b| a.id.cmp(&b.id));
        for (local, h) in houses.iter().enumerate() {
            let expected = format!("house-{:02}", realm_index * 3 + local + 1);
            assert_eq!(h.id, expected, "realm {realm_id} local {local}");
        }
    }
    validate_population(&world, &pop).expect("invariants");
}

#[test]
fn seats_unique_in_realm_and_first_is_capital() {
    let d = generate_dynastic_world(1).expect("dynastic");
    for realm in &d.world.realms {
        let mut houses: Vec<_> = d
            .population
            .houses
            .iter()
            .filter(|h| h.realm_id == realm.id)
            .collect();
        houses.sort_by(|a, b| a.id.cmp(&b.id));
        assert_eq!(houses[0].seat_territory_id, realm.capital_territory_id);
        let mut seats = BTreeSet::new();
        for h in &houses {
            assert!(
                realm.territory_ids.contains(&h.seat_territory_id),
                "seat outside realm"
            );
            assert!(seats.insert(h.seat_territory_id.as_str()));
        }
    }
}

#[test]
fn parent_graph_same_house_previous_generation_no_cycle() {
    let d = generate_dynastic_world(2).expect("dynastic");
    let by_id: BTreeMap<_, _> = d
        .population
        .persons
        .iter()
        .map(|p| (p.id.as_str(), p))
        .collect();
    for p in &d.population.persons {
        let mut seen = BTreeSet::new();
        for parent_id in &p.known_parent_ids {
            assert_ne!(parent_id, &p.id, "self-parent");
            assert!(seen.insert(parent_id.as_str()), "dup parent");
            let parent = by_id[parent_id.as_str()];
            assert_eq!(parent.house_id, p.house_id);
            match p.generation {
                GenerationBand::Elder => panic!("elder has parent"),
                GenerationBand::Current => assert_eq!(parent.generation, GenerationBand::Elder),
                GenerationBand::Young => assert_eq!(parent.generation, GenerationBand::Current),
            }
        }
        match p.generation {
            GenerationBand::Elder => assert!(p.known_parent_ids.is_empty()),
            GenerationBand::Current | GenerationBand::Young => {
                assert!(!p.known_parent_ids.is_empty());
            }
        }
    }
}

#[test]
fn ruler_links_match_ruling_house_heads() {
    let d = generate_dynastic_world(1).expect("dynastic");
    assert_eq!(d.population.ruler_links.len(), 6);
    let person_by_id: BTreeMap<_, _> = d
        .population
        .persons
        .iter()
        .map(|p| (p.id.as_str(), p))
        .collect();
    let mut linked_persons = BTreeSet::new();
    for ruler in &d.world.rulers {
        let link = d
            .population
            .ruler_links
            .iter()
            .find(|l| l.ruler_id == ruler.id)
            .expect("link");
        assert!(linked_persons.insert(link.person_id.as_str()));
        let person = person_by_id[link.person_id.as_str()];
        assert_eq!(person.name, ruler.name);
        assert_eq!(person.realm_id, ruler.realm_id);
        assert_eq!(person.generation, GenerationBand::Current);
        let realm = d
            .world
            .realms
            .iter()
            .find(|r| r.id == ruler.realm_id)
            .expect("realm");
        assert_eq!(person.home_territory_id, realm.capital_territory_id);
        let mut houses: Vec<_> = d
            .population
            .houses
            .iter()
            .filter(|h| h.realm_id == realm.id)
            .collect();
        houses.sort_by(|a, b| a.id.cmp(&b.id));
        assert_eq!(houses[0].head_person_id, person.id);
        assert_eq!(houses[0].seat_territory_id, realm.capital_territory_id);
    }
}

#[test]
fn same_seed_equality_and_seed_1_2_differ() {
    let a = generate_dynastic_world(1).expect("a");
    let b = generate_dynastic_world(1).expect("b");
    assert_eq!(a, b);
    let ba = a.to_compact_json_bytes().unwrap();
    let bb = b.to_compact_json_bytes().unwrap();
    assert_eq!(ba, bb);

    let c = generate_dynastic_world(2).expect("c");
    assert_ne!(a, c);
    assert_ne!(ba, c.to_compact_json_bytes().unwrap());
}

#[test]
fn population_does_not_change_world_skeleton_bytes() {
    for seed in [1u64, 2] {
        let world = generate_world(seed).expect("world");
        let dynastic = generate_dynastic_world(seed).expect("dynastic");
        assert_eq!(world, dynastic.world);
        assert_eq!(
            world.to_compact_json_bytes().unwrap(),
            dynastic.world.to_compact_json_bytes().unwrap()
        );
        validate_world(&dynastic.world).expect("world ok");
        validate_population(&dynastic.world, &dynastic.population).expect("pop ok");
    }
}

#[test]
fn person_ids_contiguous_per_house() {
    let d = generate_dynastic_world(0).expect("dynastic");
    for (i, house) in d.population.houses.iter().enumerate() {
        let base = i * 8;
        for (m, mid) in house.member_ids.iter().enumerate() {
            let expected = format!("person-{:03}", base + m + 1);
            assert_eq!(mid, &expected);
        }
    }
}

#[test]
fn full_invariants_on_representative_seeds() {
    for seed in [0u64, 1, 2, 42, u64::MAX] {
        let d = generate_dynastic_world(seed).expect("dynastic");
        validate_population(&d.world, &d.population).expect("invariants");
    }
}

// population-check 1/2의 exact 회귀는 common::CLI_EXACT_REGRESSION이 담당한다.

#[test]
fn cli_population_1_succeeds() {
    common::assert_cli_json_eq(
        &["population", "1"],
        &generate_dynastic_world(1).expect("dynastic 1"),
    );
}

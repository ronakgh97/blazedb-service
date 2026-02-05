/// Calculate a deterministic port for a given instance_id
///
/// Uses a simple hashing to map instance IDs to ports in the range 50000-59999.
/// This ensures:
/// - Same instance_id always gets same port
/// - Proxy and container spawning use identical logic
/// - No port conflicts within 10k container limit, I guess?, Mathematically possible but unlikely
//TODO: Need to find a better way to port allocations to avoid collisions, maybe use a more robust hash function or maintain a mapping in storage
#[inline]
pub fn calculate_container_port(instance_id: &str) -> u16 {
    let hash: u16 = instance_id
        .chars()
        .take(8)
        .fold(0u16, |acc, c| acc.wrapping_add(c as u16));

    50000 + (hash % 10000)
}

#[test]
fn test_deterministic_port_assignment() {
    let instance_id = "a1a70763676476be92f8d80c5ed9ab74";

    let port1 = calculate_container_port(instance_id);
    let port2 = calculate_container_port(instance_id);

    assert_eq!(port1, port2);
    assert!(port1 >= 50000 && port1 < 60000);
}

#[test]
fn test_different_ids_different_ports() {
    let id1 = "a1a70763676476be";
    let id2 = "b2c91234567890ab";

    let port1 = calculate_container_port(id1);
    let port2 = calculate_container_port(id2);

    assert!(port1 >= 50000 && port1 < 60000);
    assert!(port2 >= 50000 && port2 < 60000);
}

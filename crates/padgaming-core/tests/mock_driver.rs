//! O core deve compilar e ser testável só com o `MockDriver` (T-201).

use padgaming_driver::{MockDriver, SlotId, VirtualPadDriver, XReport, xbuttons};

#[test]
fn works_against_the_trait_object() {
    let driver: Box<dyn VirtualPadDriver> = Box::new(MockDriver::default());
    let slot = SlotId::new(0).unwrap();
    let pad = driver.plug(slot).unwrap();
    let report = XReport {
        buttons: xbuttons::A,
        ..XReport::default()
    };
    driver.update(&pad, &report).unwrap();
    assert_eq!(driver.user_index(&pad), Some(0));
    driver.unplug(pad).unwrap();
}

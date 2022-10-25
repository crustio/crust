# Steps to integrate XStorage pallet into your collator

## Add the following dependency into your runtime
```rust
xstorage = { git = "https://github.com/paritytech/crust", branch = "parachain/shadow", package= "xstorage", default-features = false }
```

## Add the following code into your runtime
```rust
pub struct Preparator;
impl xstorage::PrepareStorageOrder for Preparator {
	fn prepare_storage_order(cid: Vec<u8>, size: u64) -> Vec<u8> {
		RuntimeCall::Xstorage(xstorage::RuntimeCall::inner_place_storage_order(cid, size)).encode()
	}
}

impl xstorage::Config for Runtime {
	type HrmpMessageSender = ParachainSystem;
	type Preparator = Preparator;
	type DoPlaceStorageOrder = ();
}
```

## Register your parachain id
You can open an issue to tell us your parachain id so that our chain can accept your cross chain place storage order.
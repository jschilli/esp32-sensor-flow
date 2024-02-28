#

## Building

```sh
CRATE_CC_NO_DEFAULTS=1 cargo build
```

TODO:

- [ ] Unit tests running
- [ ]

### Next Steps

Redo this to have ButtonState be a simple read into a struct like

```rust
struct Buttons {
    menu: bool,
    play: bool,
    vol_up: bool,
    vol_dn: bool
}
```

Then a ButtonFlow is over `Buttons` for a given button, then a window to capture previous and current value, then `goes_active` and `goes_inactive`

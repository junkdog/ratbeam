# ratbeam

A [ratatui](https://github.com/ratatui/ratatui) backend using [beamterm](https://github.com/junkdog/beamterm) for native, GPU-accelerated terminal rendering via OpenGL 3.3.


## Usage

The backend does not own the window or GL context. Applications provide an `Rc<glow::Context>` and a `TerminalGrid`; ratbeam only bridges the ratatui API.

```rust
let backend = BeamtermBackend::new(grid, gl.clone());
let mut terminal = Terminal::new(backend)?;

terminal.draw(|frame| {
    // use ratatui as usual
})?;
```

See `examples/demo` and `examples/wave-interference` for full working examples with glutin+winit windowing.

## License

MIT

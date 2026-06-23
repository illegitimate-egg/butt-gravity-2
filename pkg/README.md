Wasm Build:
> [!NOTE]
> WebGl is dead

> [!NOTE]
> WebGPU support is temperamental, Chrome (150) supports immediates as of 2026-06-17,
> but, at the time of writing, no other browser engine supports WebGPU immediates.
> Even then, immediates aren't guaranteed to work on all version of chrome 150.
> Perhaps it's better to just run the desktop application.
> It's a shame that I don't have prebuilt versions for that.
> Maybe I should like do that.
> Hint hint
> Maybe I should do that

`wasm-pack build --target web --dev`

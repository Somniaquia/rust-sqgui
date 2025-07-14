# wgpu GUI Library Roadmap

## Core Rendering Infrastructure
- [ ] **Render Pipeline**
    - [ ] Base render pass configuration
    - [ ] Texture binding system
    - [ ] Material system (shaders, uniforms)
    - [ ] Depth/stencil management
- [ ] **Resource Management**
    - [ ] Texture atlas/allocator
    - [ ] GPU resource pool
    - [ ] Shader hot-reloading

## 2D Rendering Foundation
- [ ] **SpriteBatch System**
    - [ ] Batch rendering (quads)
    - [ ] Dynamic batcing
    - [ ] 9-patch sprite rendering
    - [ ] SDF font rendering
- [ ] **Primitive Drawing**
    - [ ] Rectangles (filled/outline)
    - [ ] Circles/rounded rects
    - [ ] Lines/curves

  ## UI Framework
- [ ] **Layout System**
    - [ ] macros for declarative UI initialization
    - [ ] 2-pass compositing/positioning 
    - [ ] scroll rendertargets and overflow handling
    - [ ] graph-based positioning? force graphs/static
- [ ] **Input Handling**
    - [ ] Focus management / hit testing
    - [ ] Input event bubbling / callback based alternative
    - [ ] Gestures like drag/drop, pinch/zoom
- [ ] **UI Components**
    - [ ] basic Panel/container type
    - [ ] Button
    - [ ] Text input
    - [ ] Sliders/scrollbars
    - [ ] Checkbox/radio
    - [ ] Dropdown/menus
    - [ ] Shortcuts, arrow key actions
    - [ ] **Text Rendering**
        - [ ] Font metrics
        - [ ] Text layout (wrapping/alignment)
        - [ ] Rich text formatting
- [ ] **Optimization**
    - [ ] Dirty region rendering!!!
    - [ ] GPU occlusion culling
- [ ] **Theming**
    - [ ] 9-patch custom textures
    - [ ] Color schemes
    - [ ] Border styles
    - [ ] Shadows/blur/effects
    - [ ] Transitional animations!!!
    - [ ] Runtime theme switching


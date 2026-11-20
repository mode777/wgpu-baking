# Project: Browser-Based Global Illumination Baker

## Objective
Fork the existing `lightbaker` project and evolve it into a browser-based, GPU-accelerated lightmap baking application.

The application should allow users to create and edit a 3D scene in the browser, populate it with assets from [polyfork.dev](https://polyfork.dev/), generate lightmap UVs automatically, and bake high-quality global illumination directly in the browser using WebGPU.

The long-term visual quality target is comparable to **The Witness (Jonathan Blow, 2016)**: richly colored indirect illumination, convincing ambient occlusion, soft shadows, multiple-bounce diffuse global illumination, and beautiful color bleeding across simple geometric environments.

This should be a real, functional implementation, not just a UI mockup or a static demonstration.

---

# 1. Research and technical background
The implementation should build upon the existing lightbaker project:

[https://github.com/jer-nc/lightbaker](https://github.com/jer-nc/lightbaker)

Important findings from the initial research:

### Existing lightbaker architecture
The project is implemented in Rust and uses wgpu and WGSL for GPU computation.

Its architecture includes:

- BVH-based ray tracing.
- GPU-accelerated lighting computations.
- Multiple indirect-lighting bounces.
- Irradiance volumes.
- Tile-based incremental baking.
- A chart-generation and lightmap-layout implementation.
- A TypeScript-facing interface.
It is particularly interesting because its architecture is already designed around browser technologies and WebGPU.

Preserve as much of the existing architecture as possible. Avoid unnecessarily replacing its rendering or ray-tracing implementation.

### Important limitations to address
The existing lightbaker implementation has several limitations that should be addressed:

- Its UV layout implementation uses its own chart generation and planar projections rather than a general-purpose UV unwrapping algorithm.
- Its documented lighting inputs include directional sunlight and sky illumination.
- Point lights and spotlights are not currently exposed as general scene-light types.
- Emissive materials are not currently exposed as a complete GI-emitting material feature.
These should be treated as extension points for the project.

### The Witness research
Use these articles as important algorithmic references:

- Precomputed Lighting: https://blog.selfshadow.com/publications/precomputed-lighting/
- Hemicube Rendering and Integration: https://blog.selfshadow.com/publications/hemicube/
- Irradiance Caching: https://blog.selfshadow.com/publications/irradiance-caching/
Also investigate the open-source irradiance caching code released by Ignacio Castaño in connection with The Witness research.

An important distinction: The Witness's approach involves hemicube-based irradiance evaluation and irradiance caching. These concepts are related to, but not identical to, lightbaker's BVH-based ray-tracing and tile interpolation techniques.

The objective is to achieve comparable visual results, not necessarily reproduce the original implementation.

---

# 2. Technology stack
Use the following architecture:

### Core baking engine

- Rust
- wgpu
- WGSL
- WebAssembly (`wasm32-unknown-unknown`)
- wasm-bindgen
- wasm-pack or an equivalent WASM packaging workflow

### UV generation

- xatlas
- Integrate xatlas into the Rust baking pipeline.
- Generate lightmap UV coordinates automatically.
- Support configurable chart padding, texel density and atlas resolution.
- Preserve original texture UV coordinates independently from generated lightmap UV coordinates.
Repository: [https://github.com/jpcy/xatlas](https://github.com/jpcy/xatlas)

Investigate existing Rust bindings or implement a small FFI wrapper around the C++ library if necessary.

### Browser application

- TypeScript
- Three.js
- WebGPU
- React for application UI
- Vite for development and production builds
The viewer should use Three.js for scene presentation and the Rust/wgpu implementation for light baking.

Avoid implementing a second independent lighting engine in JavaScript.

---

# 3. WASM and WebGPU integration
A major technical objective is compiling the Rust baking engine to WebAssembly while retaining GPU acceleration through wgpu.

Target:

- `wasm32-unknown-unknown`
- Browser-based WebGPU
- WGSL compute shaders
- GPU BVH traversal and lighting calculations
Important implementation considerations:

1. Verify that the existing wgpu version and dependencies support the WASM target.
2. Implement asynchronous browser-compatible GPU initialization.
3. Avoid blocking native execution patterns that are incompatible with browsers.
4. Audit dependencies for filesystem, threading and native-only functionality.
5. Avoid relying on Rayon or native multithreading in the browser.
6. Implement appropriate browser memory management.
7. Keep large intermediate lighting buffers on the GPU wherever possible.
8. Minimize GPU-to-CPU readbacks.
9. Support incremental baking and cancellation.
10. Handle WebGPU device loss and adapter initialization failures gracefully.
WebGPU availability varies by browser, operating system and GPU. Provide a capability check and a clear fallback message when baking is unavailable.

WebGL can optionally be used for scene viewing, but should not be treated as a replacement for the WebGPU baking backend.

---

# 4. UV unwrapping integration
Replace or supplement lightbaker's existing chart-layout implementation with xatlas.

Required workflow:

1. Import geometry.
2. Extract positions, indices and normals.
3. Generate lightmap UV charts using xatlas.
4. Pack the charts into one or more lightmap atlases.
5. Apply configurable padding to avoid light bleeding.
6. Transfer the generated UVs back to the Three.js geometry.
7. Pass the UV coordinates to the baking engine.
Important:

- Keep original material UVs intact.
- Support meshes with multiple material groups.
- Support multiple lightmap atlases.
- Avoid excessive fragmentation of UV charts.
- Allow configurable texel density.
- Visualize UV charts and atlas packing in the application.
The existing lightbaker chart layout can remain available as an optional alternative if it provides useful performance or quality characteristics.

---

# 5. Lighting system
Extend the current lighting model to support a general-purpose scene lighting system.

## Direct lighting
Implement:

- Directional lights / sun.
- Point lights.
- Spotlights.
- Area lights, if feasible.
- Environment and sky illumination.
Point lights should support:

- Position.
- RGB color.
- Intensity.
- Range or physically motivated attenuation.
Spotlights should support:

- Position.
- Direction.
- Inner cone angle.
- Outer cone angle.
- Color.
- Intensity.
Area lights should ideally support rectangular and disk-shaped emitters with stochastic sampling.

## Emissive materials
Implement proper emissive geometry support.

Material properties:

- Albedo.
- Emission color.
- Emission intensity.
- Roughness, if supported by the material model.
- Transparency or opacity where appropriate.
Emissive surfaces must contribute to indirect illumination, not merely appear bright in the final rendered image.

When a GI ray hits an emissive surface, emitted radiance must be included in the lighting calculation.

Use a physically meaningful diffuse transport model. For a Lambertian surface:

`Lo = Le + (albedo / PI) * E`

where:

- `Lo` is outgoing radiance.
- `Le` is emitted radiance.
- `E` is incoming irradiance.
Support color bleeding from emissive surfaces onto nearby geometry.

---

# 6. Global illumination quality
This is the most important part of the project.

The goal is to achieve a visual result reminiscent of The Witness, rather than simply producing ambient occlusion or direct-light shadows.

Prioritize:

### Multiple-bounce diffuse GI

- Support configurable bounce counts.
- Accumulate indirect illumination.
- Prevent energy explosion and excessive brightness.
- Use a physically meaningful energy-conserving diffuse BRDF.

### Color bleeding
Colored surfaces must illuminate neighbouring surfaces.

Examples:

- A red wall should create subtle red indirect illumination on a nearby white wall.
- A green floor should contribute green bounce light to surrounding geometry.
- Emissive objects should illuminate nearby surfaces.

### Soft shadows

- Directional light with configurable angular radius.
- Soft shadows from area lights.
- Appropriate filtering and denoising.

### Ambient occlusion

- High-quality contact shadows.
- Configurable AO intensity and distance.
- Avoid excessive darkening.

### Irradiance caching and interpolation
Investigate the existing lightbaker tile-based interpolation.

Consider adding an irradiance caching implementation inspired by Ignacio Castaño's published research.

Avoid computing expensive lighting samples independently for every lightmap texel when interpolation can safely be used.

Make interpolation quality configurable and ensure that discontinuities around geometry boundaries do not cause light leaks.

### Quality settings
Provide at least three presets:

- Preview: fast, low sample count, low resolution.
- Balanced: reasonable interactive baking quality.
- High Quality: higher sample count, more bounces, higher-resolution lightmaps and improved denoising.
Expose the individual parameters for advanced users.

---

# 7. Browser demo application
Build a polished interactive demo application.

The application should feel like a lightweight 3D editor rather than a technical shader demo.

## Main viewport
Use Three.js to display the scene.

Features:

- Orbit controls.
- Object selection.
- Transform gizmos.
- Scene hierarchy.
- Material editing.
- Lighting visualization.
- Toggle between unbaked and baked lighting.
- Toggle lightmap display.
- Show UV charts.
- Display bake progress.

## Asset library: polyfork.dev
Integrate an asset browser that allows users to populate the scene with assets from polyfork.dev.

Investigate the site's available asset delivery mechanisms, APIs and licensing conditions before implementing the integration.

Do not assume an API exists without verifying it.

If direct integration is not possible, provide an importer for compatible asset formats such as GLTF/GLB.

Asset requirements:

- Preserve mesh geometry.
- Preserve original material information where possible.
- Support multiple meshes and material groups.
- Allow users to select assets and add them to the scene.
- Provide simple primitive geometry as an alternative.
Create a visually appealing example environment using a collection of colorful, stylized, low-poly assets.

The scene should contain:

- Buildings.
- Walls.
- Floors.
- Rocks.
- Trees.
- Vegetation.
- Simple architectural structures.
- Colorful geometric props.
Prefer simple geometry and solid material colors over heavily textured assets.

## Lighting editor
Allow users to add, select and modify:

- Directional lights.
- Point lights.
- Spotlights.
- Emissive materials.
Provide intuitive controls for light color, intensity, position and other relevant properties.

Allow users to modify lighting and rebake the scene.

## Bake controls
Provide:

- Atlas resolution.
- Texel density.
- Bounce count.
- Sampling quality.
- Shadow quality.
- AO strength.
- Denoising.
- Incremental baking.
Show progress, elapsed time and estimated remaining work where possible.

Support cancelling a bake.

---

# 8. Demo scene and visual target
Create a dedicated showcase environment inspired by The Witness.

The scene should demonstrate the strengths of baked global illumination.

Visual characteristics:

- Colorful solid materials.
- Simple geometric architecture.
- Low-poly vegetation.
- Large colored surfaces.
- Clean, stylized shapes.
- Minimal texture dependence.
- Strong natural sunlight.
- Soft shadows.
- Subtle ambient occlusion.
- Multiple-bounce indirect illumination.
- Visible color bleeding.
Include several carefully designed lighting demonstrations:

### Test 1: Color bleeding
Place red, green and blue walls near neutral-colored surfaces.

The indirect lighting should visibly transfer their colors onto neighbouring geometry.

### Test 2: Emissive lighting
Place emissive objects inside a relatively dark environment.

They should illuminate surrounding geometry through indirect light transport.

### Test 3: Point lights
Place several colored point lights in an architectural environment.

Demonstrate direct illumination, shadowing and indirect color contribution.

### Test 4: Multiple bounces
Create an enclosed environment with colored walls and demonstrate progressively stronger indirect illumination as the bounce count increases.

### Test 5: Outdoor environment
Create a stylized outdoor scene with directional sunlight, sky illumination, trees, rocks and architecture.

Demonstrate soft shadows and natural indirect lighting.

The visual target should be recognizable as a modern stylized, fully baked global illumination environment, not a generic physically based rendering demo.

---

# 9. Export and runtime rendering
The final baked lightmaps must be usable in Three.js.

Implement:

- HDR lightmap output where supported.
- Appropriate linear color-space handling.
- Correct UV assignment.
- Lightmap texture filtering.
- Atlas padding and dilation.
- Mipmap-safe handling.
- Correct baked-lighting material configuration.
Make sure the final renderer does not accidentally apply lighting twice.

The baked output should be exportable together with scene geometry and material information, preferably as a GLB-compatible scene or a documented asset bundle.

---

# 10. Development strategy
Work incrementally. Do not attempt to implement all features simultaneously.

## Phase 1: Understand and fork

- Inspect the entire lightbaker repository.
- Understand its GPU architecture and shader pipeline.
- Identify its current scene format.
- Identify the existing chart-generation implementation.
- Identify its native/browser build setup.
- Establish a clean fork with reproducible builds.
Deliverable: a documented technical architecture and working baseline.

## Phase 2: WASM/WebGPU

- Compile the existing project for WebAssembly.
- Initialize wgpu in the browser.
- Run the existing baking functionality on a small test scene.
- Verify actual GPU execution rather than CPU fallback.
Deliverable: a browser-based proof of concept that bakes a simple scene.

## Phase 3: xatlas

- Integrate xatlas.
- Generate lightmap UVs.
- Bake the resulting geometry.
- Verify atlas correctness and prevent light leaks.
Deliverable: automatic UV unwrapping and functional baking.

## Phase 4: Lighting extensions
Implement:

1. Point lights.
2. Emissive materials.
3. Spotlights.
4. Area lights.
5. Improved multiple-bounce GI.
Deliverable: a more general-purpose lightmapper.

## Phase 5: Demo application

- Build the Three.js scene editor.
- Integrate polyfork.dev assets or compatible asset imports.
- Add lighting controls.
- Add bake controls and progress reporting.
- Implement baked/unbaked comparison.
Deliverable: a functional interactive browser application.

## Phase 6: Visual quality and optimization

- Tune the GI implementation.
- Improve interpolation.
- Improve filtering and denoising.
- Optimize memory use and GPU performance.
- Create the final showcase environment.
- Test across supported browsers and GPUs.
Deliverable: a polished demo showcasing high-quality baked global illumination.

---

# 11. Engineering principles
A few important requirements:

- Do not replace wgpu with a JavaScript rendering implementation.
- Do not silently fall back to CPU baking.
- Keep the Rust baking engine independent of the React application.
- Keep the browser interface independent of the underlying GI algorithm.
- Use well-defined TypeScript interfaces for scene data and bake configuration.
- Keep the project buildable on native platforms as well as WASM where feasible.
- Include reproducible build instructions.
- Add automated tests for UV generation, lighting calculations and bake output.
- Add small deterministic test scenes for visual regression.
- Document all major architectural changes.
- Avoid implementing fake lighting features that only change the final appearance without participating in light transport.
When a feature is too ambitious, first implement a correct minimal version, then optimize it.

---

# 12. Definition of success
The project is successful when:

1. The Rust lightbaker engine compiles to WASM.
2. WebGPU is used for actual GPU-accelerated baking in a browser.
3. xatlas generates valid lightmap UVs.
4. A scene can be populated with imported low-poly assets.
5. Point lights and emissive materials contribute to baked GI.
6. Multiple-bounce indirect lighting works.
7. Color bleeding is clearly visible.
8. Baked lightmaps render correctly in Three.js.
9. Users can modify lighting and rebake without reloading the application.
10. The showcase environment demonstrates visual quality reminiscent of The Witness.
**Start by inspecting the existing repository and producing a technical feasibility report. Identify the exact files, modules and dependencies that need to change. Then implement Phase 1 and Phase 2 before proceeding. Do not build the entire application before verifying that the existing baker works in the browser with WebGPU.**
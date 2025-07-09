# TODO: Enhanced wgpu-code-toy Integration and Single Toy Renderer Path

## Objective
Implement production-quality wgpu-code-toy preprocessing system and eliminate dual-path rendering to create a single, powerful toy renderer path with comprehensive WGSL auto-injection features.

## Phase 0: Enhanced Preprocessor System (NEW - Root Cause Resolution)

### 0.1 Enhanced Preprocessor Directives
- [ ] **Update `src/toy/pp.rs:150-250`** - Add comprehensive preprocessor directive support including `#include <std/...>`, `#storage name type`, `#workgroup_count name x y z`, `#dispatch_once name`, `#dispatch_count name x`, `#assert predicate`, and `#data name u32 value1,value2` directives in the process_line method. This resolves the buffer size mismatch by enabling dynamic binding generation. DO NOT MOCK, FABRICATE, FAKE or SIMULATE ANY OPERATION or DATA. Make ONLY THE MINIMAL, SURGICAL CHANGES required. Do not modify or rewrite any portion of the app outside scope.

- [ ] **Act as an Objective QA Rust developer** - Review the preprocessor directive implementation for completeness. Verify all directives are properly parsed and stored in SourceMap. Confirm the implementation follows wgpu-code-toy patterns exactly. Rate the work performed previously on these requirements.

### 0.2 Standard Library Support
- [ ] **Create `src/toy/include/std/` directory with `noise.wgsl`, `math.wgsl`, `color.wgsl`, `string.wgsl`** - Implement standard library files that can be included via `#include <std/...>` directives. These provide essential utility functions for production-quality shaders. DO NOT MOCK, FABRICATE, FAKE or SIMULATE ANY OPERATION or DATA. Make ONLY THE MINIMAL, SURGICAL CHANGES required. Do not modify or rewrite any portion of the app outside scope.

- [ ] **Act as an Objective QA Rust developer** - Review the standard library implementation for functionality. Verify all files contain proper WGSL syntax and useful functions. Confirm the include system can properly load these files. Rate the work performed previously on these requirements.

### 0.3 Dynamic Binding Generation
- [ ] **Update `src/toy/bind.rs:1-200`** - Add dynamic binding generation methods that create storage buffers from `#storage` directives, user data arrays from `#data` directives, and assert counter buffers. This directly addresses the buffer size error by matching shader expectations. DO NOT MOCK, FABRICATE, FAKE or SIMULATE ANY OPERATION or DATA. Make ONLY THE MINIMAL, SURGICAL CHANGES required. Do not modify or rewrite any portion of the app outside scope.

- [ ] **Act as an Objective QA Rust developer** - Review the dynamic binding generation for accuracy. Verify storage buffers are created with correct sizes and types. Confirm all binding layouts match shader requirements. Rate the work performed previously on these requirements.

### 0.4 Enhanced Prelude Generation
- [ ] **Update `src/toy/mod.rs:141-183`** - Enhance the prelude() method to generate dynamic Custom struct from user floats, dynamic Data struct from user arrays, and add utility functions like passStore(), passLoad(), passSampleLevelBilinearRepeat(), and proper assert() function with runtime monitoring. DO NOT MOCK, FABRICATE, FAKE or SIMULATE ANY OPERATION or DATA. Make ONLY THE MINIMAL, SURGICAL CHANGES required. Do not modify or rewrite any portion of the app outside scope.

- [ ] **Act as an Objective QA Rust developer** - Review the prelude generation for completeness. Verify all structs are properly generated and all utility functions are included. Confirm the prelude integrates seamlessly with user shaders. Rate the work performed previously on these requirements.

### 0.5 Source Mapping & Error Handling
- [ ] **Update `src/toy/pp.rs:48-94` and `src/toy/mod.rs:88-138`** - Add comprehensive source mapping to preserve line numbers through preprocessing, WGSL compilation error parsing and remapping, and shader rollback capability on compilation errors. This provides production-quality debugging support. DO NOT MOCK, FABRICATE, FAKE or SIMULATE ANY OPERATION or DATA. Make ONLY THE MINIMAL, SURGICAL CHANGES required. Do not modify or rewrite any portion of the app outside scope.

- [ ] **Act as an Objective QA Rust developer** - Review the source mapping implementation for accuracy. Verify error messages correctly reference original source lines. Confirm rollback functionality works properly on compilation failures. Rate the work performed previously on these requirements.

### 0.6 Multi-Pass Rendering Support
- [ ] **Update `src/toy/mod.rs:196-320`** - Add multi-pass rendering support in render_to_buffer method including multiple compute pipeline dispatch, texture ping-pong between passes, and per-pass workgroup configuration from preprocessor directives. This enables complex shader effects. DO NOT MOCK, FABRICATE, FAKE or SIMULATE ANY OPERATION or DATA. Make ONLY THE MINIMAL, SURGICAL CHANGES required. Do not modify or rewrite any portion of the app outside scope.

- [ ] **Act as an Objective QA Rust developer** - Review the multi-pass rendering implementation for correctness. Verify pipeline dispatch ordering and texture swapping work properly. Confirm workgroup configurations are applied correctly. Rate the work performed previously on these requirements.

## Phase 1: Remove Dual-Path Code from output_generator.rs

### 1.1 Remove EliteWebGPURenderer Import
- [ ] **Update `src/output/output_generator.rs:10`** - Remove the line `use crate::webgpu::EliteWebGPURenderer;`. DO NOT MOCK, FABRICATE, FAKE or SIMULATE ANY OPERATION or DATA. Make ONLY THE MINIMAL, SURGICAL CHANGES required. Do not modify or rewrite any portion of the app outside scope.

- [ ] **Act as an Objective QA Rust developer** - Review the import removal for completeness. Verify no other references to EliteWebGPURenderer remain in imports. Confirm the removal is surgical and minimal. Rate the work performed previously on these requirements.

### 1.2 Remove Composite Rendering Methods
- [ ] **Delete method `src/output/output_generator.rs:147-197`** - Remove the entire `render_composite_layers` method. This method implements the dual-path compositing that must be eliminated. DO NOT MOCK, FABRICATE, FAKE or SIMULATE ANY OPERATION or DATA. Make ONLY THE MINIMAL, SURGICAL CHANGES required. Do not modify or rewrite any portion of the app outside scope.

- [ ] **Act as an Objective QA Rust developer** - Review the method removal for completeness. Verify the method is entirely removed with no partial fragments remaining. Confirm no other code depends on this method. Rate the work performed previously on these requirements.

### 1.3 Remove Text Layer Rendering Method
- [ ] **Delete method `src/output/output_generator.rs:589-608`** - Remove the entire `render_text_layer` method that uses EliteWebGPURenderer. DO NOT MOCK, FABRICATE, FAKE or SIMULATE ANY OPERATION or DATA. Make ONLY THE MINIMAL, SURGICAL CHANGES required. Do not modify or rewrite any portion of the app outside scope.

- [ ] **Act as an Objective QA Rust developer** - Review the method removal for accuracy. Verify the method is completely removed. Confirm this doesn't break any remaining code. Rate the work performed previously on these requirements.

### 1.4 Remove Layer Compositing Method
- [ ] **Delete method `src/output/output_generator.rs:610-679`** - Remove the entire `composite_layers_optimized` method that blends background and text layers. DO NOT MOCK, FABRICATE, FAKE or SIMULATE ANY OPERATION or DATA. Make ONLY THE MINIMAL, SURGICAL CHANGES required. Do not modify or rewrite any portion of the app outside scope.

- [ ] **Act as an Objective QA Rust developer** - Review the compositing method removal. Verify all alpha blending logic is removed. Confirm the method deletion is complete. Rate the work performed previously on these requirements.

### 1.5 Remove EliteWebGPURenderer from Live Preview
- [ ] **Update `src/output/output_generator.rs:312-322`** - Remove the EliteWebGPURenderer creation and testing code in `launch_live_preview`. Delete lines creating and testing the elite_renderer. DO NOT MOCK, FABRICATE, FAKE or SIMULATE ANY OPERATION or DATA. Make ONLY THE MINIMAL, SURGICAL CHANGES required. Do not modify or rewrite any portion of the app outside scope.

- [ ] **Act as an Objective QA Rust developer** - Review the live preview changes. Verify all EliteWebGPURenderer references are removed. Confirm the remaining code still functions correctly. Rate the work performed previously on these requirements.

## Phase 2: Refactor to Single Toy Renderer Path

### 2.1 Rewrite Generate Method
- [ ] **Update `src/output/output_generator.rs:102-145`** - Rewrite the `generate` method to use only the enhanced toy renderer with full preprocessor support. Keep syntax highlighting and layout. For static outputs, create toy renderer directly, optionally render text to storage buffer via #storage directive, render to buffer, and save output. Remove all references to composite rendering. DO NOT MOCK, FABRICATE, FAKE or SIMULATE ANY OPERATION or DATA. Make ONLY THE MINIMAL, SURGICAL CHANGES required. Do not modify or rewrite any portion of the app outside scope.

- [ ] **Act as an Objective QA Rust developer** - Review the generate method rewrite. Verify it uses only the enhanced toy renderer path with full preprocessor support. Confirm all composite rendering logic is removed. Verify the method handles all output formats correctly. Rate the work performed previously on these requirements.

### 2.2 Rename Background Layer Method
- [ ] **Update `src/output/output_generator.rs:199-254`** - Rename `render_background_layer` method to `render_with_enhanced_toy` to reflect the enhanced preprocessor capabilities. Update the method signature to remove the "background" terminology and support the new preprocessor features. DO NOT MOCK, FABRICATE, FAKE or SIMULATE ANY OPERATION or DATA. Make ONLY THE MINIMAL, SURGICAL CHANGES required. Do not modify or rewrite any portion of the app outside scope.

- [ ] **Act as an Objective QA Rust developer** - Review the method rename for consistency. Verify all callers are updated to use the new name. Confirm the method logic integrates properly with enhanced preprocessor features. Rate the work performed previously on these requirements.

### 2.3 Update Method Calls
- [ ] **Update all callers** - Find and update all places that call the renamed method to use the new name `render_with_enhanced_toy`. DO NOT MOCK, FABRICATE, FAKE or SIMULATE ANY OPERATION or DATA. Make ONLY THE MINIMAL, SURGICAL CHANGES required. Do not modify or rewrite any portion of the app outside scope.

- [ ] **Act as an Objective QA Rust developer** - Review all method call updates. Verify no calls to the old method name remain. Confirm all updates are correct and complete. Rate the work performed previously on these requirements.

## Phase 3: Integration Testing

### 3.1 Test Enhanced Bandwidth Shader Rendering
- [ ] **Test `just skew wgsl/bandwidth.wgsl`** - Run the command and verify it renders without the "Buffer is bound with size 16 where the shader expects 32" error using the enhanced preprocessor system. Verify the output is created successfully with full preprocessor support. DO NOT MOCK, FABRICATE, FAKE or SIMULATE ANY OPERATION or DATA. Make ONLY THE MINIMAL, SURGICAL CHANGES required. Do not modify or rewrite any portion of the app outside scope.

- [ ] **Act as an Objective QA Rust developer** - Review the test results. Verify the buffer size error is resolved through enhanced binding generation. Confirm the shader renders correctly with full preprocessor support. Check that the output file is valid. Rate the work performed previously on these requirements.

### 3.2 Test Text-Enabled Shader with Storage Directives
- [ ] **Test text rendering with enhanced unified shader** - Test a shader that uses text rendering via #storage directive approach with the enhanced preprocessor to ensure text functionality is preserved and improved. DO NOT MOCK, FABRICATE, FAKE or SIMULATE ANY OPERATION or DATA. Make ONLY THE MINIMAL, SURGICAL CHANGES required. Do not modify or rewrite any portion of the app outside scope.

- [ ] **Act as an Objective QA Rust developer** - Review the text rendering test. Verify text is correctly rendered to storage buffer via #storage directive. Confirm shaders can access text data through enhanced preprocessor. Validate the output quality is production-ready. Rate the work performed previously on these requirements.

## Success Criteria

- The "Buffer is bound with size 16 where the shader expects 32" error is resolved through enhanced binding generation
- `just skew wgsl/bandwidth.wgsl` renders successfully with full preprocessor support
- All dual-path rendering code is removed
- Enhanced single toy renderer path is implemented with production-quality preprocessing
- Text rendering works via #storage directive approach
- Full wgpu-code-toy feature parity is achieved
- Code changes are minimal and surgical
- No unrelated code is modified
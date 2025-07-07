# Acknowledgements

## wgpu-compute-toy

Codeskew incorporates code from [wgpu-compute-toy](https://github.com/compute-toys/wgpu-compute-toy), which is the compute shader engine for [compute.toys](https://compute.toys). The wgpu-compute-toy code has been copied and slightly modified to fit the needs of Codeskew.

### License

wgpu-compute-toy is licensed under the MIT License:

```
MIT License

Copyright (c) 2022-2023 David A Roberts & Cornus Ammonis

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.
```

### Usage in Codeskew

We have incorporated the compute shader engine from wgpu-compute-toy to handle the rendering of code with 3D perspective effects. The implementation in Codeskew maintains the original MIT license, and this acknowledgement serves to fulfill the license requirement to include the copyright notice and permission notice in all copies or substantial portions of the software.

### Changes Made

The wgpu-compute-toy code has been modified to:

1. Integrate with the Codeskew rendering pipeline
2. Support the specific perspective transformations needed for code visualization
3. Adapt to Codeskew's output formats and requirements

### Links

- Original project: [wgpu-compute-toy](https://github.com/compute-toys/wgpu-compute-toy)
- Related web project: [compute.toys](https://compute.toys)
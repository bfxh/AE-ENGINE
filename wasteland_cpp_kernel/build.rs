// build.rs — 用 cc crate 编译 C++ SIMD 内核
//
// cc crate 会自动检测系统 C++ 编译器：
//   - Windows MSVC: cl.exe
//   - Windows GNU:  g++ (MSYS2 MinGW64)
//   - Linux:        g++ 或 clang++
//
// AVX2/FMA 编译参数: -mavx2 -mfma

fn main() {
    cc::Build::new()
        .cpp(true)                          // 启用 C++ 编译
        .file("../cpp/simd_kernels.cpp")    // C++ 源文件
        .flag_if_supported("-mavx2")        // AVX2
        .flag_if_supported("-mfma")         // FMA
        .flag_if_supported("-O3")           // 最高优化
        .flag_if_supported("-std=c++17")    // C++17 标准
        .warnings(true)
        .compile("simd_kernels");

    println!("cargo:rerun-if-changed=../cpp/simd_kernels.cpp");
}

cmake_minimum_required(VERSION 3.10)
project(NeuroBackend)

set(CMAKE_CXX_STANDARD 17)

# find_package(LLVM REQUIRED CONFIG)
# include_directories(${LLVM_INCLUDE_DIRS})
# add_definitions(${LLVM_DEFINITIONS})

add_executable(neuro_backend main.cpp LLVMEmitter.cpp)
# llvm_map_components_to_libnames(llvm_libs Core)
# target_link_libraries(neuro_backend ${llvm_libs})

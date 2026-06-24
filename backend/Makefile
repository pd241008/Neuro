CXX := g++
CXXFLAGS := -std=c++17 -I. -I/usr/local/include
LDFLAGS := $(shell pkg-config --libs protobuf 2>/dev/null || echo "-lprotobuf -L/usr/local/lib")
BUILD_DIR := build

.PHONY: all clean

all: $(BUILD_DIR)/neuro_backend

$(BUILD_DIR)/ast.pb.cc: ../shared_ast/ast.proto
	@mkdir -p $(BUILD_DIR)
	protoc --cpp_out=$(BUILD_DIR) -I../shared_ast ../shared_ast/ast.proto

$(BUILD_DIR)/ast.pb.o: $(BUILD_DIR)/ast.pb.cc
	$(CXX) $(CXXFLAGS) -c $< -o $@

$(BUILD_DIR)/main.o: main.cpp NeuroBackend.h $(BUILD_DIR)/ast.pb.h
	$(CXX) $(CXXFLAGS) -c main.cpp -o $@

$(BUILD_DIR)/LLVMEmitter.o: LLVMEmitter.cpp NeuroBackend.h $(BUILD_DIR)/ast.pb.h
	$(CXX) $(CXXFLAGS) -c LLVMEmitter.cpp -o $@

$(BUILD_DIR)/neuro_backend: $(BUILD_DIR)/main.o $(BUILD_DIR)/LLVMEmitter.o $(BUILD_DIR)/ast.pb.o
	$(CXX) $(CXXFLAGS) $^ -o $@ $(LDFLAGS)

clean:
	rm -rf $(BUILD_DIR)

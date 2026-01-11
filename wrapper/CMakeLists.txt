cmake_minimum_required(VERSION 3.20)
project(EPBotWrapper LANGUAGES CXX)

# This project requires Visual Studio with C++/CLI support
# Build with: cmake -G "Visual Studio 17 2022" -A x64 ..
#             cmake --build . --config Release

set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_STANDARD_REQUIRED ON)

# EPBot DLL location (relative to wrapper directory)
set(EPBOT_DLL_DIR "${CMAKE_SOURCE_DIR}/..")

# Source files
set(SOURCES
    src/epbot_wrapper.cpp
)

# Header files
set(HEADERS
    include/epbot_ffi.h
)

# Create the wrapper DLL
add_library(EPBotWrapper SHARED ${SOURCES} ${HEADERS})

# C++/CLI settings - these are applied via target properties for VS
set_target_properties(EPBotWrapper PROPERTIES
    # Enable CLR support (managed C++)
    COMMON_LANGUAGE_RUNTIME ""
    VS_DOTNET_TARGET_FRAMEWORK_VERSION "v4.8"
    VS_GLOBAL_CLRSupport "true"
    VS_GLOBAL_ManagedAssembly "true"

    # Output settings
    OUTPUT_NAME "EPBotWrapper"
    RUNTIME_OUTPUT_DIRECTORY "${CMAKE_BINARY_DIR}/bin"
    LIBRARY_OUTPUT_DIRECTORY "${CMAKE_BINARY_DIR}/lib"
)

# Compile options for C++/CLI
target_compile_options(EPBotWrapper PRIVATE
    /clr        # Enable Common Language Runtime
    /EHa        # Enable C++ exceptions with SEH
    /MD         # Use multi-threaded DLL runtime
)

# Preprocessor definitions
target_compile_definitions(EPBotWrapper PRIVATE
    EPBOT_WRAPPER_EXPORTS
    _UNICODE
    UNICODE
)

# Include directories
target_include_directories(EPBotWrapper PUBLIC
    ${CMAKE_CURRENT_SOURCE_DIR}/include
)

# Add reference to EPBot64.dll
# Note: In C++/CLI, we use #using directive in source code
# The DLL needs to be in the same directory or in the assembly search path

# Copy EPBot DLLs to output directory
add_custom_command(TARGET EPBotWrapper POST_BUILD
    COMMAND ${CMAKE_COMMAND} -E copy_if_different
        "${EPBOT_DLL_DIR}/EPBot64.dll"
        "$<TARGET_FILE_DIR:EPBotWrapper>"
    COMMAND ${CMAKE_COMMAND} -E copy_if_different
        "${EPBOT_DLL_DIR}/EPBot64.tlb"
        "$<TARGET_FILE_DIR:EPBotWrapper>"
    COMMENT "Copying EPBot64.dll to output directory"
)

# Install rules
install(TARGETS EPBotWrapper
    LIBRARY DESTINATION lib
    RUNTIME DESTINATION bin
    ARCHIVE DESTINATION lib
)

install(FILES include/epbot_ffi.h DESTINATION include)

# Also install EPBot DLLs
install(FILES
    "${EPBOT_DLL_DIR}/EPBot64.dll"
    "${EPBOT_DLL_DIR}/EPBot64.tlb"
    DESTINATION bin
)

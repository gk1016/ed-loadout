cmake_minimum_required(VERSION 3.21)
project(loadout LANGUAGES CXX)

set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
set(CMAKE_AUTOMOC ON)

# Qt Quick + Controls (LGPL). Widgets only for the native folder dialog
# (IMPORT FOLDER, SETTINGS). No Network: LOADOUT talks to nothing.
find_package(Qt6 6.7 REQUIRED COMPONENTS Core Gui Widgets Quick Qml QuickControls2)

# ---------------------------------------------------------------- rust core
# The same crate builds the static library linked below and the CLI
# (rust/target/<profile>/ed-loadout.exe), which package.ps1 ships beside it.
set(RUST_DIR      "${CMAKE_CURRENT_SOURCE_DIR}/rust")
set(RUST_TARGET   "${RUST_DIR}/target")
set(RUST_PROFILE  "$<IF:$<CONFIG:Debug>,debug,release>")
set(RUST_LIB      "${RUST_TARGET}/${RUST_PROFILE}/loadout_core.lib")
set(CARGO_FLAGS   "$<IF:$<CONFIG:Debug>,,--release>")

add_custom_target(rust_core ALL
    COMMAND cargo build ${CARGO_FLAGS}
    WORKING_DIRECTORY "${RUST_DIR}"
    BYPRODUCTS "${RUST_LIB}"
    COMMENT "cargo build (loadout-core)"
    USES_TERMINAL
    VERBATIM
)

qt_standard_project_setup()

qt_add_executable(loadout
    icons/app.rc
    cpp/main.cpp
    cpp/core.cpp
    cpp/core.h
)

# Build identity from the crate version - one place, not two that drift.
file(READ "${RUST_DIR}/Cargo.toml" _cargo_toml)
string(REGEX MATCH "\nversion *= *\"([0-9a-zA-Z.\\-]+)\"" _m "${_cargo_toml}")
set(LOADOUT_VERSION "${CMAKE_MATCH_1}")
if (NOT LOADOUT_VERSION)
    message(FATAL_ERROR "could not read version from ${RUST_DIR}/Cargo.toml")
endif()
string(TIMESTAMP LOADOUT_BUILD_DATE "%Y-%m-%d %H:%MZ" UTC)
execute_process(COMMAND git rev-parse --short HEAD
                WORKING_DIRECTORY ${CMAKE_SOURCE_DIR}
                OUTPUT_VARIABLE LOADOUT_GIT_SHA
                OUTPUT_STRIP_TRAILING_WHITESPACE
                ERROR_QUIET)
if (NOT LOADOUT_GIT_SHA)
    set(LOADOUT_GIT_SHA "nogit")
endif()
execute_process(COMMAND git status --porcelain
                WORKING_DIRECTORY ${CMAKE_SOURCE_DIR}
                OUTPUT_VARIABLE _git_dirty
                ERROR_QUIET)
if (_git_dirty)
    set(LOADOUT_GIT_SHA "${LOADOUT_GIT_SHA}+")
endif()
message(STATUS "LOADOUT version ${LOADOUT_VERSION} built ${LOADOUT_BUILD_DATE} @ ${LOADOUT_GIT_SHA}")

# Everything a shell target needs: identity, the bridge header, Qt, one build
# of the core.
function(loadout_shell target rust_lib rust_target)
    target_compile_definitions(${target} PRIVATE
        LOADOUT_VERSION="${LOADOUT_VERSION}"
        LOADOUT_BUILD_DATE="${LOADOUT_BUILD_DATE}"
        LOADOUT_GIT_SHA="${LOADOUT_GIT_SHA}"
    )
    add_dependencies(${target} ${rust_target})
    target_include_directories(${target} PRIVATE
        "${CMAKE_CURRENT_SOURCE_DIR}/cpp"
        "${RUST_TARGET}/cxxbridge"
    )
    target_compile_options(${target} PRIVATE
        $<$<CXX_COMPILER_ID:MSVC>:/W4 /permissive->
    )
    target_link_libraries(${target} PRIVATE
        Qt6::Core Qt6::Gui Qt6::Widgets Qt6::Quick Qt6::Qml Qt6::QuickControls2
        "${rust_lib}"
    )
    if (WIN32)
        # What Rust std and the facts backend pull in (dxgi is raw-dylib).
        target_link_libraries(${target} PRIVATE
            ws2_32 userenv advapi32 bcrypt ntdll synchronization user32 gdi32)
    endif()
    # GUI subsystem: double-clicking LOADOUT opens a window, never a console.
    set_target_properties(${target} PROPERTIES WIN32_EXECUTABLE TRUE)
endfunction()

loadout_shell(loadout "${RUST_LIB}" rust_core)

# QML, fonts and icons beside the binary (edit .qml, re-run; no rebuild).
add_custom_command(TARGET loadout POST_BUILD
    COMMAND ${CMAKE_COMMAND} -E copy_directory
        "${CMAKE_CURRENT_SOURCE_DIR}/qml" "$<TARGET_FILE_DIR:loadout>/qml"
    COMMAND ${CMAKE_COMMAND} -E copy_directory
        "${CMAKE_CURRENT_SOURCE_DIR}/fonts" "$<TARGET_FILE_DIR:loadout>/fonts"
    COMMAND ${CMAKE_COMMAND} -E make_directory "$<TARGET_FILE_DIR:loadout>/icons"
    COMMAND ${CMAKE_COMMAND} -E copy_if_different
        "${CMAKE_CURRENT_SOURCE_DIR}/icons/loadout.ico"
        "${CMAKE_CURRENT_SOURCE_DIR}/icons/loadout.svg"
        "$<TARGET_FILE_DIR:loadout>/icons"
    COMMENT "staging qml/ fonts/ icons/"
)

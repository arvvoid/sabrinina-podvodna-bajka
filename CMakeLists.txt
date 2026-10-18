cmake_minimum_required(VERSION 3.20)
project(sabrina LANGUAGES CXX)
set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
option(SABRINA_BUILD_GAME "Build the Godot native extension" ON)
include(CTest)
# godot-cpp selects the MSVC CRT through CMAKE_MSVC_RUNTIME_LIBRARY.
# Configure it before creating any native targets so the core, tests and
# extension all use the same runtime. Standalone builds retain CMake defaults.
if(SABRINA_BUILD_GAME)
  set(GODOTCPP_TARGET template_release CACHE STRING "" FORCE)
  set(GODOTCPP_BUILD_PROFILE "${CMAKE_SOURCE_DIR}/build_profile.json" CACHE PATH "" FORCE)
  add_subdirectory(third_party/godot-cpp)
endif()
add_library(reef_simulation STATIC
  src/simulation/world.cpp src/simulation/simulation.cpp
  src/simulation/systems/player.cpp src/simulation/systems/encounters.cpp
  src/simulation/systems/magic.cpp src/simulation/systems/wave.cpp
  src/simulation/systems/fish.cpp src/simulation/systems/contacts.cpp src/simulation/systems/jellyfish.cpp)
set_target_properties(reef_simulation PROPERTIES POSITION_INDEPENDENT_CODE ON)
target_include_directories(reef_simulation PUBLIC src)
add_executable(simulation_tests tests/simulation_tests.cpp)
target_link_libraries(simulation_tests PRIVATE reef_simulation)
target_include_directories(simulation_tests PRIVATE src)
add_test(NAME simulation COMMAND simulation_tests)
add_executable(entity_tests tests/entity_tests.cpp)
target_link_libraries(entity_tests PRIVATE reef_simulation)
add_test(NAME entity_lifecycle COMMAND entity_tests)
add_executable(contact_tests tests/contact_tests.cpp)
target_link_libraries(contact_tests PRIVATE reef_simulation)
add_test(NAME soft_contacts COMMAND contact_tests)
add_executable(simulation_trace tests/simulation_trace.cpp)
target_link_libraries(simulation_trace PRIVATE reef_simulation)
add_executable(balance_report tests/balance_report.cpp)
target_link_libraries(balance_report PRIVATE reef_simulation)
if(SABRINA_BUILD_GAME)
  add_library(sabrina SHARED src/register_types.cpp src/presentation/reef_game.cpp src/presentation/reef_world.cpp src/presentation/reef_characters.cpp src/presentation/reef_ui.cpp src/presentation/reef_audio.cpp src/presentation/runtime_checks.cpp src/presentation/shader_library.cpp src/presentation/character_assets.cpp src/presentation/reef_assets.cpp src/presentation/crystal_visuals.cpp)
  target_include_directories(sabrina PRIVATE src)
  target_link_libraries(sabrina PRIVATE godot-cpp reef_simulation)
  if(MSVC)
    target_compile_options(sabrina PRIVATE /utf-8)
  endif()
  set_target_properties(sabrina PROPERTIES
    LIBRARY_OUTPUT_DIRECTORY "${CMAKE_SOURCE_DIR}/game/bin"
    RUNTIME_OUTPUT_DIRECTORY "${CMAKE_SOURCE_DIR}/game/bin")
  foreach(config DEBUG RELEASE RELWITHDEBINFO MINSIZEREL)
    set_target_properties(sabrina PROPERTIES
      LIBRARY_OUTPUT_DIRECTORY_${config} "${CMAKE_SOURCE_DIR}/game/bin"
      RUNTIME_OUTPUT_DIRECTORY_${config} "${CMAKE_SOURCE_DIR}/game/bin")
  endforeach()
endif()

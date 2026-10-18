// SPDX-License-Identifier: Apache-2.0
#pragma once
#include "character_assets.h"
#include "simulation/simulation.h"
#include <godot_cpp/classes/audio_stream.hpp>
#include <godot_cpp/classes/audio_stream_player.hpp>
#include <godot_cpp/classes/button.hpp>
#include <godot_cpp/classes/camera3d.hpp>
#include <godot_cpp/classes/config_file.hpp>
#include <godot_cpp/classes/control.hpp>
#include <godot_cpp/classes/directional_light3d.hpp>
#include <godot_cpp/classes/h_slider.hpp>
#include <godot_cpp/classes/input_event.hpp>
#include <godot_cpp/classes/label.hpp>
#include <godot_cpp/classes/mesh_instance3d.hpp>
#include <godot_cpp/classes/multi_mesh_instance3d.hpp>
#include <godot_cpp/classes/node3d.hpp>
#include <godot_cpp/classes/omni_light3d.hpp>
#include <godot_cpp/classes/progress_bar.hpp>
#include <godot_cpp/classes/rich_text_label.hpp>
#include <godot_cpp/classes/shader_material.hpp>
#include <godot_cpp/classes/sphere_mesh.hpp>
#include <godot_cpp/classes/standard_material3d.hpp>
#include <vector>
namespace godot {
class ReefGame : public Node3D {
    GDCLASS(ReefGame, Node3D)
    struct FishVisual {
        MeshInstance3D *shield = nullptr;
        Node3D *root = nullptr;
        RigVisual rig;
        reef::Entity entity{};
        bool red = false;
        float yaw = 0;
        int species = -1;
    };
    struct JellyVisual {
        Node3D *root = nullptr;
        RigVisual rig;
    };
    CharacterLibrary character_library;
    RigVisual sabrina_rig;
    int hand_bone_left = -1, hand_bone_right = -1;
    MeshInstance3D *wave_visual = nullptr;
    bool wave_pending = false;
    bool assets_failed = false;
    reef::World sim;
    reef::Settings game_settings;
    Camera3D *camera = nullptr;
    DirectionalLight3D *sun = nullptr;
    Node3D *reef_environment = nullptr;
    Node3D *mermaid = nullptr, *crystal = nullptr, *crystal_ring = nullptr, *spark_root = nullptr;
    OmniLight3D *magic_light = nullptr;
    std::array<FishVisual, reef::fish_count> fish_visuals;
    std::array<JellyVisual, reef::jelly_count> jelly_visuals;
    std::vector<Node3D *> shafts, sparks, song_rings, song_notes;
    float scenery_time = 0;
    Ref<ShaderMaterial> water_volume;
    std::vector<Ref<ShaderMaterial>> reef_materials;
    bool loading = true;
    int load_stage = 0;
    float load_time = 0;
    Control *loading_ui = nullptr, *loading_cover = nullptr;
    Label *loading_status = nullptr;
    ProgressBar *loading_bar = nullptr;
    void build_loading();
    void finish_loading();
    void build_detailed_fish();
    void add_water_details();
    void build_sabrina();
    void build_jellyfish();
    void update_characters(float dt, float motion);
    void build_distant_life();
    MultiMeshInstance3D *plankton = nullptr, *fish_pops = nullptr;
    Ref<StandardMaterial3D> damage_material;
    bool siren_red = false;
    std::vector<Node3D *> distant_fish;
    std::vector<Ref<ShaderMaterial>> distant_materials;
    Ref<SphereMesh> sphere_mesh;
    Control *ui = nullptr, *menu = nullptr, *hud = nullptr, *pause_panel = nullptr, *settings_panel = nullptr,
            *info_panel = nullptr, *gameover_panel = nullptr;
    Label *title = nullptr, *subtitle = nullptr, *intro = nullptr, *stats = nullptr, *phase_label = nullptr,
          *hint = nullptr, *notice = nullptr, *caption = nullptr, *score_card = nullptr,
          *record_label = nullptr;
    Label *settings_title = nullptr, *music_label = nullptr, *effects_label = nullptr, *voice_label = nullptr,
          *rest_label = nullptr, *info_title = nullptr, *pause_title = nullptr, *pause_body = nullptr;
    RichTextLabel *info_text = nullptr;
    Button *play_button = nullptr, *aquarium_button = nullptr, *credits_button = nullptr,
           *settings_button = nullptr, *help_button = nullptr, *language_button = nullptr,
           *quit_button = nullptr, *resume_button = nullptr, *home_button = nullptr, *song_button = nullptr,
           *wave_button = nullptr;
    Button *fullscreen_button = nullptr, *quality_button = nullptr, *motion_button = nullptr,
           *gentle_button = nullptr, *visitors_button = nullptr, *settings_done = nullptr,
           *original_project_button = nullptr, *help_replay = nullptr, *help_pause = nullptr,
           *info_close = nullptr;
    HSlider *music_slider = nullptr, *effects_slider = nullptr, *voice_slider = nullptr,
            *rest_slider = nullptr;
    Label *health_label = nullptr, *gameover_title = nullptr, *gameover_body = nullptr;
    Button *retry_button = nullptr, *gameover_home = nullptr;
    ProgressBar *health_bar = nullptr;
    ProgressBar *magic_bar = nullptr, *song_bar = nullptr, *wave_bar = nullptr;
    Label *magic_status = nullptr, *song_status = nullptr, *wave_status = nullptr;
    AudioStreamPlayer *effects = nullptr, *voice = nullptr, *music = nullptr, *visit_music = nullptr,
                      *song = nullptr;
    bool started = false, paused = false, pointer_mode = false, fullscreen = true, reduced_motion = false,
         info_is_help = false;
    bool capture_mode = false, smoke_mode = false, verify_mode = false, verify_failed = false,
         headless = false;
    int language = 0, best = 0, quality = 1, verify_frame = 0, smoke_steps = 0;
    float elapsed = 0, notice_time = 0, score_time = 0, capture_time = 0, verify_value = 0, voice_gain = .9f,
          music_gain = .30f, effects_gain = .55f;
    double verify_rig_time = 0;
    Vector2 pointer;
    Node3D *crystal_model = nullptr;
    MeshInstance3D *crystal_wake = nullptr;
    std::vector<Ref<ShaderMaterial>> crystal_materials;
    std::array<Vector3, 24> trail{};
    bool crystal_was_flying = false;
    float crystal_trail_time = 0;
    Vector3 crystal_previous{};
    void build_crystal();
    void update_crystal(float dt);
    std::vector<Node3D *> trail_nodes;
    String capture_kind;
    bool initial_fullscreen = false;
    Ref<StandardMaterial3D> material(Color color, float emission = 0);
    MeshInstance3D *ellipsoid(Node3D *parent, Vector3 pos, Vector3 scale, Color color, float glow = 0);
    MeshInstance3D *cone(Node3D *parent, Vector3 pos, float bottom, float top, float height, Color color);
    MeshInstance3D *ring(Node3D *parent, Vector3 pos, float radius, Color color);
    MeshInstance3D *fin(Node3D *parent, Vector3 a, Vector3 b, Vector3 c, Color color);
    void branch(Node3D *parent, Vector3 a, Vector3 b, float radius, Color color);
    void build_world();
    void build_characters();
    void build_ui();
    void build_audio();
    void setup_input();
    void update_visuals(float dt);
    void update_hud();
    void update_audio(float dt);
    void refresh_text();
    void apply_graphics();
    void play_effect(const String &name, float pitch = 1);
    void play_voice(const String &name, const String &subtitle, bool interrupt = false);
    Ref<AudioStream> load_audio(const String &name);
    Ref<AudioStream> compose_music(bool visit);
    void save_settings();
    void load_settings();
    void verify_input();
    void show_notice(const String &text, float seconds = 4);
    String tr3(const char *en, const char *hr, const char *it) const;
    void begin(bool aquarium);

  protected:
    static void _bind_methods();

  public:
    void _ready() override;
    void _exit_tree() override;
    void _process(double delta) override;
    void _physics_process(double delta) override;
    void _unhandled_input(const Ref<InputEvent> &event) override;
    void start_game();
    void start_aquarium();
    void toggle_pause();
    void go_home();
    void cycle_language();
    void toggle_song();
    void cast_wave();
    void open_settings();
    void open_credits();
    void open_original_project();
    void open_help();
    void close_panel();
    void replay_help();
    void pause_help();
    void quit_game();
    void toggle_fullscreen();
    void cycle_quality();
    void toggle_motion();
    void toggle_gentle();
    void cycle_visitors();
    void set_music_volume(double v);
    void set_effects_volume(double v);
    void set_voice_volume(double v);
    void set_rest_time(double v);
};
} // namespace godot

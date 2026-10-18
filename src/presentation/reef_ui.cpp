// SPDX-License-Identifier: MIT
#include "reef_game.h"
#include <godot_cpp/classes/canvas_layer.hpp>
#include <godot_cpp/classes/file_access.hpp>
#include <godot_cpp/classes/font_file.hpp>
#include <godot_cpp/classes/image.hpp>
#include <godot_cpp/classes/image_texture.hpp>
#include <godot_cpp/classes/os.hpp>
#include <godot_cpp/classes/panel.hpp>
#include <godot_cpp/classes/style_box_flat.hpp>
#include <godot_cpp/classes/style_box_texture.hpp>
#include <godot_cpp/classes/texture_rect.hpp>
#include <godot_cpp/classes/theme.hpp>
using namespace godot;
namespace {
Color ink(.87, .97, 1), gold(1, .81, .45), cyan(.25, .87, .87);
Ref<StyleBoxFlat> box(Color c, int r = 18) {
    Ref<StyleBoxFlat> s;
    s.instantiate();
    s->set_bg_color(c);
    s->set_corner_radius_all(r);
    s->set_content_margin_all(12);
    return s;
}
Ref<Texture2D> reef_texture(const String &name) {
    Ref<Image> image;
    image.instantiate();
    image->load_svg_from_string(FileAccess::get_file_as_string("res://assets/ui/reef/" + name + ".svg"), 1.0);
    return ImageTexture::create_from_image(image);
}
Ref<StyleBoxTexture> reef_frame(const String &name, float margin = 28) {
    Ref<StyleBoxTexture> style;
    style.instantiate();
    style->set_texture(reef_texture(name));
    style->set_texture_margin_all(margin);
    style->set_content_margin_all(12);
    return style;
}
void reef_icon(Control *parent, const String &name, Vector2 pos, Vector2 size) {
    auto *icon = memnew(TextureRect);
    parent->add_child(icon);
    icon->set_texture(reef_texture(name));
    icon->set_expand_mode(TextureRect::EXPAND_IGNORE_SIZE);
    icon->set_stretch_mode(TextureRect::STRETCH_KEEP_ASPECT_CENTERED);
    icon->set_position(pos);
    icon->set_size(size);
    icon->set_mouse_filter(Control::MOUSE_FILTER_IGNORE);
}
void readable_theme(Control *root) {
    Ref<FontFile> font;
    font.instantiate();
    font->load_dynamic_font("res://assets/fonts/DejaVuSans.ttf");
    Ref<Theme> theme;
    theme.instantiate();
    theme->set_default_font(font);
    theme->set_default_font_size(20);
    root->set_theme(theme);
}
Label *label(Control *p, Vector2 pos, Vector2 size, int fs = 22) {
    auto *l = memnew(Label);
    p->add_child(l);
    l->set_position(pos);
    l->set_size(size);
    l->add_theme_font_size_override("font_size", fs);
    l->add_theme_color_override("font_color", ink);
    l->set_mouse_filter(Control::MOUSE_FILTER_IGNORE);
    return l;
}
Control *layer(Control *p) {
    auto *c = memnew(Control);
    p->add_child(c);
    c->set_anchors_and_offsets_preset(Control::PRESET_FULL_RECT);
    c->set_mouse_filter(Control::MOUSE_FILTER_IGNORE);
    return c;
}
void panel(Control *p, Vector2 pos, Vector2 size) {
    auto *c = memnew(Panel);
    p->add_child(c);
    c->set_position(pos);
    c->set_size(size);
    if (size.y < 45)
        c->add_theme_stylebox_override("panel", box(Color(.025, .17, .22, .94), 16));
    else
        c->add_theme_stylebox_override("panel", reef_frame(size.y < 150 ? "compact" : "panel", 26));
    c->set_mouse_filter(Control::MOUSE_FILTER_IGNORE);
    if (size.y > 200) {
        reef_icon(p, "shell", {pos.x + size.x / 2 - 42, pos.y - 38}, {84, 84});
        reef_icon(p, "coral", {pos.x - 36, pos.y + size.y - 115}, {88, 126});
        reef_icon(p, "star", {pos.x + size.x - 32, pos.y + size.y - 62}, {64, 64});
    }
}
Button *button(Control *p, Vector2 pos, Vector2 size, ReefGame *owner, const char *method,
               bool primary = false) {
    auto *b = memnew(Button);
    p->add_child(b);
    b->set_position(pos);
    b->set_size(size);
    b->add_theme_font_size_override("font_size", 20);
    b->add_theme_stylebox_override("normal", reef_frame(primary ? "button" : "button_teal", 22));
    b->add_theme_stylebox_override("hover", reef_frame("button_hover", 22));
    b->add_theme_stylebox_override("pressed", reef_frame("button_down", 22));
    b->add_theme_stylebox_override("disabled", reef_frame("button_teal", 22));
    b->add_theme_color_override("font_disabled_color", Color(.52, .71, .72));
    auto f = box(Color(0, 0, 0, 0), 12);
    f->set_border_width_all(3);
    f->set_border_color(gold);
    b->add_theme_stylebox_override("focus", f);
    b->add_theme_color_override("font_color", primary ? Color(.20, .17, .18) : ink);
    b->add_theme_color_override("font_focus_color", primary ? Color(.20, .17, .18) : ink);
    b->add_theme_color_override("font_hover_color", Color(.20, .17, .18));
    b->add_theme_color_override("font_pressed_color", Color(.20, .17, .18));
    b->connect("pressed", Callable(owner, method));
    return b;
}
HSlider *slider(Control *p, Vector2 pos, ReefGame *owner, const char *method, double value, double lo = 0,
                double hi = 100) {
    auto *s = memnew(HSlider);
    p->add_child(s);
    s->set_position(pos);
    s->set_size(Vector2(400, 34));
    s->set_min(lo);
    s->set_max(hi);
    s->set_step(1);
    s->set_value(value);
    s->connect("value_changed", Callable(owner, method));
    return s;
}
} // namespace
void ReefGame::build_ui() {
    auto *c = memnew(CanvasLayer);
    add_child(c);
    ui = memnew(Control);
    c->add_child(ui);
    ui->set_anchors_and_offsets_preset(Control::PRESET_FULL_RECT);
    ui->set_mouse_filter(Control::MOUSE_FILTER_IGNORE);
    readable_theme(ui);
    menu = layer(ui);
    panel(menu, {64, 65}, {535, 775});
    subtitle = label(menu, {100, 94}, {450, 30}, 17);
    subtitle->add_theme_color_override("font_color", gold);
    title = label(menu, {100, 137}, {460, 155}, 44);
    intro = label(menu, {100, 340}, {450, 76}, 21);
    intro->set_autowrap_mode(TextServer::AUTOWRAP_WORD_SMART);
    play_button = button(menu, {100, 430}, {460, 58}, this, "start_game", true);
    credits_button = button(menu, {100, 510}, {220, 52}, this, "open_credits");
    settings_button = button(menu, {340, 510}, {220, 52}, this, "open_settings");
    aquarium_button = button(menu, {100, 584}, {460, 52}, this, "start_aquarium");
    help_button = button(menu, {100, 658}, {220, 52}, this, "open_help");
    language_button = button(menu, {340, 658}, {220, 52}, this, "cycle_language");
    quit_button = button(menu, {100, 742}, {220, 48}, this, "quit_game");
    record_label = label(menu, {340, 742}, {220, 48}, 17);
    hud = layer(ui);
    panel(hud, {24, 22}, {490, 100});
    reef_icon(hud, "shell", {469, 76}, {58, 58});
    phase_label = label(hud, {48, 42}, {450, 32}, 24);
    stats = label(hud, {48, 80}, {450, 28}, 18);
    wave_button = button(hud, {1020, 24}, {135, 50}, this, "cast_wave");
    song_button = button(hud, {1170, 24}, {225, 50}, this, "toggle_song");
    auto *p = button(hud, {1415, 24}, {160, 50}, this, "toggle_pause");
    p->set_text("II");
    panel(hud, {210, 864}, {1180, 34});
    hint = label(hud, {210, 864}, {1180, 32}, 17);
    hint->set_horizontal_alignment(HORIZONTAL_ALIGNMENT_CENTER);
    auto resource = [&](int x, Color colour, ProgressBar *&bar, Label *&text) {
        panel(hud, {float(x), 790}, {340, 70});
        text = label(hud, {float(x + 20), 807}, {300, 29}, 17);
        bar = memnew(ProgressBar);
        hud->add_child(bar);
        bar->set_position({float(x + 20), 836});
        bar->set_size({300, 10});
        bar->set_show_percentage(false);
        auto bg = box(Color(.02, .13, .2, .8), 5);
        bg->set_content_margin_all(0);
        auto fill = box(colour, 5);
        fill->set_content_margin_all(0);
        bar->add_theme_stylebox_override("background", bg);
        bar->add_theme_stylebox_override("fill", fill);
        bar->set_size({300, 10});
    };
    reef_icon(hud, "shell", {240, 779}, {58, 58});
    reef_icon(hud, "star", {1315, 806}, {58, 58});
    resource(270, Color(.55, .80, 1), magic_bar, magic_status);
    resource(630, Color(1, .58, .85), song_bar, song_status);
    resource(990, Color(.38, 1, .76), wave_bar, wave_status);
    panel(hud, {600, 22}, {400, 92});
    reef_icon(hud, "heart", {610, 36}, {56, 56});
    health_label = label(hud, {679, 37}, {300, 32}, 20);
    health_bar = memnew(ProgressBar);
    hud->add_child(health_bar);
    health_bar->set_position({679, 73});
    health_bar->set_size({290, 18});
    health_bar->set_show_percentage(false);
    auto health_fill = box(Color(1, .40, .48), 8);
    health_fill->set_content_margin_all(0);
    auto health_bg = box(Color(.10, .24, .29), 8);
    health_bg->set_content_margin_all(0);
    health_bar->add_theme_stylebox_override("fill", health_fill);
    health_bar->add_theme_stylebox_override("background", health_bg);
    health_bar->set_size({290, 18});
    score_card = label(hud, {500, 130}, {600, 90}, 21);
    score_card->set_horizontal_alignment(HORIZONTAL_ALIGNMENT_CENTER);
    notice = label(hud, {350, 700}, {900, 48}, 23);
    notice->set_horizontal_alignment(HORIZONTAL_ALIGNMENT_CENTER);
    notice->add_theme_color_override("font_shadow_color", Color(0, .07, .12));
    notice->add_theme_constant_override("shadow_offset_x", 2);
    notice->add_theme_constant_override("shadow_offset_y", 2);
    caption = label(ui, {330, 744}, {940, 40}, 20);
    caption->set_horizontal_alignment(HORIZONTAL_ALIGNMENT_CENTER);
    caption->add_theme_color_override("font_shadow_color", Color(0, 0, 0));
    caption->add_theme_constant_override("shadow_offset_x", 2);
    caption->add_theme_constant_override("shadow_offset_y", 2);
    pause_panel = layer(ui);
    panel(pause_panel, {500, 190}, {600, 510});
    pause_title = label(pause_panel, {550, 230}, {500, 65}, 40);
    pause_body = label(pause_panel, {550, 305}, {500, 45}, 21);
    resume_button = button(pause_panel, {550, 385}, {500, 60}, this, "toggle_pause", true);
    auto *ps = button(pause_panel, {550, 465}, {500, 60}, this, "open_settings");
    ps->set_text(String::utf8("⚙"));
    home_button = button(pause_panel, {550, 545}, {500, 60}, this, "go_home");
    settings_panel = layer(ui);
    panel(settings_panel, {270, 70}, {1060, 760});
    settings_title = label(settings_panel, {315, 103}, {970, 60}, 36);
    fullscreen_button = button(settings_panel, {315, 190}, {450, 52}, this, "toggle_fullscreen");
    quality_button = button(settings_panel, {815, 190}, {465, 52}, this, "cycle_quality");
    motion_button = button(settings_panel, {315, 260}, {450, 52}, this, "toggle_motion");
    gentle_button = button(settings_panel, {815, 260}, {465, 52}, this, "toggle_gentle");
    music_label = label(settings_panel, {315, 345}, {450, 35});
    music_slider = slider(settings_panel, {315, 393}, this, "set_music_volume", music_gain * 100);
    effects_label = label(settings_panel, {315, 445}, {450, 35});
    effects_slider = slider(settings_panel, {315, 493}, this, "set_effects_volume", effects_gain * 100);
    voice_label = label(settings_panel, {315, 545}, {450, 35});
    voice_slider = slider(settings_panel, {315, 593}, this, "set_voice_volume", voice_gain * 100);
    rest_label = label(settings_panel, {815, 345}, {465, 35});
    rest_slider =
        slider(settings_panel, {815, 393}, this, "set_rest_time", game_settings.rest_seconds, 25, 150);
    visitors_button = button(settings_panel, {815, 465}, {465, 58}, this, "cycle_visitors");
    settings_done = button(settings_panel, {815, 725}, {465, 58}, this, "close_panel", true);
    info_panel = layer(ui);
    panel(info_panel, {230, 60}, {1140, 780});
    info_title = label(info_panel, {280, 93}, {1040, 65}, 38);
    info_text = memnew(RichTextLabel);
    info_panel->add_child(info_text);
    info_text->set_position({280, 180});
    info_text->set_size({1040, 525});
    info_text->add_theme_font_size_override("normal_font_size", 23);
    info_text->add_theme_color_override("default_color", ink);
    original_project_button = button(info_panel, {280, 745}, {640, 52}, this, "open_original_project");
    help_replay = button(info_panel, {280, 745}, {310, 52}, this, "replay_help");
    help_pause = button(info_panel, {610, 745}, {310, 52}, this, "pause_help");
    info_close = button(info_panel, {940, 745}, {380, 52}, this, "close_panel", true);
    gameover_panel = layer(ui);
    gameover_panel->set_mouse_filter(Control::MOUSE_FILTER_STOP);
    panel(gameover_panel, {440, 210}, {720, 470});
    reef_icon(gameover_panel, "heart", {746, 240}, {108, 90});
    gameover_title = label(gameover_panel, {485, 332}, {630, 62}, 38);
    gameover_title->set_horizontal_alignment(HORIZONTAL_ALIGNMENT_CENTER);
    gameover_body = label(gameover_panel, {490, 405}, {620, 70}, 21);
    gameover_body->set_autowrap_mode(TextServer::AUTOWRAP_WORD_SMART);
    gameover_body->set_horizontal_alignment(HORIZONTAL_ALIGNMENT_CENTER);
    retry_button = button(gameover_panel, {490, 500}, {620, 62}, this, "start_game", true);
    gameover_home = button(gameover_panel, {490, 586}, {620, 56}, this, "go_home");
    gameover_panel->hide();
}
void ReefGame::refresh_text() {
    gameover_title->set_text(tr3("Game over", "Kraj igre", "Fine della partita"));
    gameover_body->set_text(tr3("Sabrina needs a rest. Try again when you are ready.",
                                "Sabrini treba odmor. Pokušaj ponovno kada budeš spreman.",
                                "Sabrina ha bisogno di riposo. Riprova quando vuoi."));
    retry_button->set_text(tr3("Swim again", "Zaplivaj ponovno", "Nuota ancora"));
    gameover_home->set_text(tr3("Back to our reef", "Natrag na naš greben", "Torna alla barriera"));
    title->set_text(tr3("Sabrina's\nUnderwater\nFairy Tale", "Sabrinina\npodvodna\nbajka",
                        "La fiaba\nsottomarina\ndi Sabrina"));
    subtitle->set_text(
        tr3("A REMAKE BY LUKA DEZULOVIC", "PRERADA: LUKA DEZULOVIC", "UN REMAKE DI LUKA DEZULOVIC"));
    intro->set_text(tr3("A living sea. A mischievous jellyfish.\nAnd a little mermaid with a magic crystal.",
                        "Živo more. Nestašna meduza.\nI mala sirena s čarobnim kristalom.",
                        "Un mare vivo. Una medusa birichina.\nE una sirenetta con un cristallo magico."));
    play_button->set_text(tr3("Let's play", "Igrajmo se", "Giochiamo"));
    aquarium_button->set_text(tr3("Quiet aquarium · no jellyfish", "Mirni akvarij · bez meduza",
                                  "Acquario tranquillo · senza meduse"));
    credits_button->set_text(tr3("Credits", "Autori", "Crediti"));
    settings_button->set_text(tr3("Settings", "Postavke", "Impostazioni"));
    help_button->set_text(tr3("How to play", "Kako igrati", "Come giocare"));
    language_button->set_text(language == 0 ? "English" : language == 1 ? "Hrvatski" : "Italiano");
    quit_button->set_text(tr3("Goodbye", "Doviđenja", "Arrivederci"));
    record_label->set_text(tr3("Best visit: ", "Najbolji posjet: ", "Visita migliore: ") +
                           String::num_int64(best));
    pause_title->set_text(tr3("A little pause", "Mali predah", "Una piccola pausa"));
    pause_body->set_text(
        tr3("Your fish will wait for you.", "Ribice će te pričekati.", "I pesciolini ti aspettano."));
    resume_button->set_text(tr3("Continue swimming", "Nastavi plivati", "Continua a nuotare"));
    home_button->set_text(tr3("Return to the beginning", "Povratak na početak", "Torna all'inizio"));
    settings_title->set_text(tr3("Make yourself at home", "Uredi svoj mali svijet", "Il tuo piccolo mondo"));
    fullscreen_button->set_text(tr3("Fullscreen: ", "Cijeli zaslon: ", "Schermo intero: ") +
                                (fullscreen ? tr3("on", "da", "sì") : tr3("off", "ne", "no")));
    quality_button->set_text(tr3("Graphics: ", "Grafika: ", "Grafica: ") +
                             (quality == 0   ? tr3("Light", "Lagana", "Leggera")
                              : quality == 1 ? tr3("Balanced", "Uravnotežena", "Bilanciata")
                                             : tr3("High", "Visoka", "Alta")));
    motion_button->set_text(reduced_motion
                                ? tr3("Motion: reduced", "Kretanje: smanjeno", "Movimento: ridotto")
                                : tr3("Motion: natural", "Kretanje: prirodno", "Movimento: naturale"));
    gentle_button->set_text(game_settings.gentle ? tr3("Gentle play", "Nježna igra", "Gioco dolce")
                                                 : tr3("Lively play", "Živahnija igra", "Gioco vivace"));
    music_label->set_text(tr3("Music", "Glazba", "Musica"));
    effects_label->set_text(
        tr3("Magic & sound effects", "Čarolija i zvučni efekti", "Magia ed effetti sonori"));
    voice_label->set_text(tr3("Original voices", "Izvorni glasovi", "Voci originali"));
    rest_label->set_text(tr3("Quiet time: ", "Vrijeme odmora: ", "Tempo tranquillo: ") +
                         String::num_int64(game_settings.rest_seconds) + " s");
    visitors_button->set_text(tr3("At most ", "Najviše ", "Al massimo ") +
                              String::num_int64(game_settings.visitor_limit) +
                              tr3(" jellyfish", " meduza", " meduse"));
    settings_done->set_text(tr3("Done", "Gotovo", "Fatto"));
    original_project_button->set_text(
        tr3("Play the original on Scratch ↗", "Izvornik na Scratchu ↗", "Gioca all’originale su Scratch ↗"));
    info_close->set_text(tr3("Back", "Natrag", "Indietro"));
    help_replay->set_text(tr3("Listen again", "Poslušaj ponovno", "Ascolta di nuovo"));
    help_pause->set_text(tr3("Pause / resume voice", "Zaustavi / nastavi glas", "Pausa / riprendi voce"));
    song_button->set_text(sim.sabrina.song.active
                              ? tr3("♪ Singing · Q / X", "♪ Pjevanje · Q / X", "♪ Canto · Q / X")
                              : tr3("♪ Sing · Q / X", "♪ Zapjevaj · Q / X", "♪ Canta · Q / X"));
}
void ReefGame::update_hud() {
    health_bar->set_value(sim.sabrina.vitality.health);
    health_label->set_text(tr3("Health · ", "Zdravlje · ", "Salute · ") +
                           String::num_int64(int(std::ceil(sim.sabrina.vitality.health))) + " / 100");
    phase_label->set_text(
        sim.session.water == reef::Water::Visit
            ? tr3("A jellyfish is visiting!", "Meduza je u posjetu!", "Una medusa in visita!")
            : tr3("A peaceful little sea", "Mirno malo more", "Un piccolo mare tranquillo"));
    stats->set_text(
        String::num_int64(reef::alive(sim)) + tr3(" fish   ·   ", " ribica   ·   ", " pesci   ·   ") +
        (sim.session.water == reef::Water::Visit
             ? tr3("Score ", "Bodovi ", "Punti ") + String::num_int64(sim.points.score)
         : sim.settings.aquarium ? tr3("Take your time", "Bez žurbe", "Senza fretta")
                                 : tr3("Next visit in ", "Sljedeći posjet za ", "Prossima visita tra ") +
                                       String::num_int64(int(sim.session.rest_remaining) + 1) + " s"));
    magic_bar->set_value(100 * (1 - sim.sabrina.cooldown / reef::recharge_duration(sim)));
    song_bar->set_value(sim.sabrina.song.energy);
    wave_bar->set_value(100 * (1 - sim.sabrina.wave.cooldown / reef::wave_recharge_duration(sim)));
    auto seconds = [](float value) {
        int n = int(std::ceil(value));
        return String::num_int64(n / 60) + ":" + (n % 60 < 10 ? "0" : "") + String::num_int64(n % 60);
    };
    magic_status->set_text(
        tr3("Crystal · ", "Kristal · ", "Cristallo · ") +
        (sim.sabrina.cooldown > 0 ? seconds(sim.sabrina.cooldown) : tr3("ready", "spreman", "pronto")));
    song_status->set_text(
        tr3("Song · ", "Pjesma · ", "Canto · ") + String::num_int64(int(sim.sabrina.song.energy)) + "%" +
        (sim.sabrina.song.locked   ? tr3(" · recharging", " · čeka punjenje", " · ricarica")
         : sim.sabrina.song.active ? String::utf8(" · ") + String::num(reef::song_radius(sim), 1) + " m"
                                   : ""));
    wave_status->set_text(tr3("Wave · ", "Val · ", "Onda · ") +
                          (sim.sabrina.wave.cooldown > 0
                               ? String::num(sim.sabrina.wave.cooldown, 1) + " s"
                               : tr3("ready · F / B", "spreman · F / B", "pronta · F / B")));
    wave_button->set_text(tr3("Wave · F", "Val · F", "Onda · F"));
    wave_button->set_disabled(sim.sabrina.wave.cooldown > 0 || sim.sabrina.vitality.stunned > 0);
    song_button->set_text(
        sim.sabrina.song.active   ? tr3("♪ Stop · Q / X", "♪ Prestani · Q / X", "♪ Ferma · Q / X")
        : sim.sabrina.song.locked ? tr3("♪ Recharging", "♪ Obnavljanje", "♪ Ricarica")
                                  : tr3("♪ Sing · Q / X", "♪ Zapjevaj · Q / X", "♪ Canta · Q / X"));
    hint->set_text(tr3("Mouse / WASD / stick · swim     Click / Space / A · crystal     Q / X · song     "
                       "RMB / F / B · wave",
                       "Miš / WASD / palica · plivaj     Klik / Razmak / A · kristal     Q / X · pjesma     "
                       "Desni klik / F / B · val",
                       "Mouse / WASD / levetta · nuota     Clic / Spazio / A · cristallo     Q / X · canto   "
                       "  Clic destro / F / B · onda"));
    notice->set_visible(notice_time > 0);
    score_card->set_visible(score_time > 0);
    score_card->set_text(
        tr3("The sea is peaceful again", "More je ponovno mirno", "Il mare è di nuovo tranquillo") + "\n+" +
        String::num_int64(sim.points.last_score) + tr3(" points · ", " bodova · ", " punti · ") +
        String::num_int64(sim.points.last_survivors) +
        tr3(" fish safe", " ribica na sigurnom", " pesci al sicuro"));
}
void ReefGame::open_settings() {
    paused = true;
    pause_panel->hide();
    menu->hide();
    settings_panel->show();
    fullscreen_button->grab_focus();
}
void ReefGame::open_credits() {
    info_is_help = false;
    paused = true;
    pause_panel->hide();
    menu->hide();
    info_panel->show();
    help_replay->hide();
    help_pause->hide();
    original_project_button->show();
    voice->stop();
    caption->set_text("");
    info_title->set_text(tr3("Credits", "Autori", "Crediti"));
    String lang = language == 1 ? "hr" : language == 2 ? "it" : "en";
    info_text->set_text(FileAccess::get_file_as_string("res://assets/credits/credits_" + lang + ".txt"));
    info_text->scroll_to_line(0);
    info_close->grab_focus();
}
void ReefGame::open_original_project() {
    OS::get_singleton()->shell_open("https://scratch.mit.edu/projects/1203591608/");
}
void ReefGame::open_help() {
    info_is_help = true;
    paused = true;
    pause_panel->hide();
    menu->hide();
    info_panel->show();
    original_project_button->hide();
    help_replay->show();
    help_pause->show();
    info_title->set_text(tr3("Swim, sing, and look after the fish", "Plivaj, pjevaj i čuvaj ribice",
                             "Nuota, canta e proteggi i pesci"));
    info_text->set_text(tr3(
        "Swim with mouse, WASD / arrows, or the left controller stick.\n\n"
        "CRYSTAL · Click, Space / E or controller A charges a crystal. It returns touched fish to Sabrina "
        "with a brief protective bubble and sends a jellyfish away. Recharge: 2:30 in Gentle mode, 3:00 in "
        "Lively. Save it for danger!\n\n"
        "SONG · Q or controller X starts or stops singing. Only fish inside the pink ripples can hear you; "
        "even distant jellyfish sometimes hear it and approach. The circle shrinks as energy runs out. A "
        "full song lasts 12 seconds "
        "(10 in Lively); full recharge takes 20 seconds (24 in Lively). Stop early to save energy. If "
        "emptied, the song stays locked until completely full.\n\n"
        "WAVE · Right-click, F or controller B sends a water wave from your hands in the direction you face. "
        "It pushes a group of fish; jellyfish are unaffected. Recharge: 3 seconds (4 in Lively). "
        "Aim away from the jellyfish; swim slowly while singing to keep your school close.\n\n"
        "Each fish first rescued in a visit earns +80 by crystal and +20 by a wave directed away from a "
        "nearby jellyfish. Escorting a threatened fish to clear water with song earns +30 once per visit. A "
        "repelled jellyfish gives "
        "+300, with 80–115 per survivor and +60 per species. Casting costs no points; a sting costs up to "
        "100. A sting costs "
        "health too. Health slowly recovers after 8 safe seconds. At zero health, the game ends.\n\n"
        "Between visits, relax as new fish arrive. Quiet aquarium has no jellyfish. Settings control rest "
        "time, visitors, graphics and sound. Esc / Start pauses; F11 switches fullscreen.",
        "Plivaj mišem, tipkama WASD / strelicama ili lijevom palicom kontrolera.\n\n"
        "KRISTAL · Klik, Razmak / E ili A puni kristal. Vraća dotaknute ribice Sabrini i tjera meduzu. "
        "Spašene ribice kratko štiti mjehurić. Obnova traje 2:30 u nježnom načinu, 3:00 u živahnom. Sačuvaj "
        "ga za opasnost!\n\n"
        "PJESMA · Q ili X uključuje i zaustavlja pjesmu. Čuju je ribice u ružičastom krugu, a ponekad i "
        "udaljene "
        "meduze. Krug se smanjuje kako nestaje energije. Puna pjesma traje 12 sekundi (10 u živahnom), "
        "obnova 20 (24 u živahnom). Stani ranije i sačuvaj energiju. Ako je potpuno potrošiš, mora se "
        "potpuno obnoviti.\n\n"
        "VAL · Desni klik, F ili B šalje val iz ruku u smjeru pogleda. Gura skupinu ribica, ali ne meduze. "
        "Obnova traje 3 sekunde (4 u živahnom). Usmjeri val od meduze; plivaj polako dok pjevaš.\n\n"
        "Prvo spašavanje ribice u posjetu donosi +80 kristalom i +20 valom koji je gura od obližnje meduze. "
        "Pjesma donosi +30 za pratnju ugrožene ribice na sigurno, jednom u posjetu. Otjerana meduza daje "
        "+300, uz "
        "80–115 po preživjeloj ribici i +60 po vrsti. Čarolija ne troši bodove, ubod oduzima do 100 i "
        "zdravlje. "
        "Nakon 8 sigurnih sekundi zdravlje se polako obnavlja. Na nuli je kraj igre.\n\n"
        "Između posjeta uživaj u miru i novim ribicama. Mirni akvarij nema meduza. Postavke mijenjaju odmor, "
        "meduze, grafiku i zvuk. Esc / Start za predah; F11 za cijeli zaslon.",
        "Nuota con mouse, WASD / frecce o levetta sinistra.\n\n"
        "CRISTALLO · Clic, Spazio / E o A carica il cristallo. Riporta i pesci toccati a Sabrina e allontana "
        "una medusa. Una bolla protegge brevemente i pesci salvati. Ricarica: 2:30 in modalità Dolce, 3:00 "
        "in Vivace. Conservalo per il pericolo!\n\n"
        "CANTO · Q o X avvia o ferma il canto. Solo i pesci nel cerchio rosa ti sentono, a volte anche le "
        "meduse "
        "lontane! Il cerchio si restringe mentre consumi energia. Un canto pieno dura 12 secondi (10 in "
        "Vivace); ricarica completa in 20 (24 in Vivace). Fermati prima per risparmiare energia. Se la "
        "esaurisci, devi aspettare la ricarica completa.\n\n"
        "ONDA · Clic destro, F o B lancia un’onda dalle mani nella direzione dello sguardo. Spinge i pesci, "
        "ma non le meduse. Ricarica: 3 secondi (4 in Vivace). Mira lontano dalla medusa e nuota piano mentre "
        "canti.\n\n"
        "Il primo salvataggio di ogni pesce nella visita vale +80 con il cristallo e +20 con un’onda che "
        "allontana il pesce da una medusa vicina. Scortare col canto un pesce in pericolo al sicuro vale +30 "
        "una volta per visita. Una "
        "medusa allontanata vale +300, più 80–115 per sopravvissuto e +60 per specie. La magia non costa "
        "punti, una "
        "puntura fino a 100. Una puntura riduce anche la salute. Dopo 8 secondi senza punture si rigenera "
        "lentamente. A zero la partita finisce.\n\n"
        "Tra le visite, rilassati mentre arrivano nuovi pesci. Acquario tranquillo non ha meduse. Regola "
        "riposo, meduse, grafica e suono nelle impostazioni. Esc / Start pausa; F11 schermo intero."));
    info_text->scroll_to_line(0);
    replay_help();
    info_close->grab_focus();
}
void ReefGame::replay_help() {
    if (!info_is_help)
        return;
    voice->set_stream_paused(false);
    play_voice(String("help_") + (language == 1 ? "hr" : language == 2 ? "it" : "en"), "", true);
}
void ReefGame::pause_help() {
    voice->set_stream_paused(!voice->get_stream_paused());
}
void ReefGame::close_panel() {
    bool was_info = info_panel->is_visible();
    settings_panel->hide();
    info_panel->hide();
    if (was_info) {
        voice->stop();
        caption->set_text("");
    }
    save_settings();
    if (started) {
        paused = true;
        pause_panel->show();
        resume_button->grab_focus();
    } else {
        paused = false;
        menu->show();
        play_button->grab_focus();
    }
}
void ReefGame::build_loading() {
    auto *canvas = memnew(CanvasLayer);
    add_child(canvas);
    canvas->set_layer(10);
    loading_ui = memnew(Control);
    canvas->add_child(loading_ui);
    loading_ui->set_anchors_and_offsets_preset(Control::PRESET_FULL_RECT);
    readable_theme(loading_ui);
    loading_cover = layer(loading_ui);
    panel(loading_cover, {0, 0}, {1600, 900});
    panel(loading_ui, {90, 115}, {730, 660});
    auto *brand = label(loading_ui, {135, 145}, {650, 40}, 18);
    brand->set_text("LUKA DEZULOVIC");
    brand->add_theme_color_override("font_color", gold);
    auto *name = label(loading_ui, {135, 205}, {640, 160}, 48);
    name->set_text(tr3("Sabrina's\nUnderwater Fairy Tale", "Sabrinina\npodvodna bajka",
                       "La fiaba sottomarina\ndi Sabrina"));
    auto *origin = label(loading_ui, {135, 400}, {640, 170}, 24);
    origin->set_autowrap_mode(TextServer::AUTOWRAP_WORD_SMART);
    origin->set_text(
        tr3("An idea by Sabrina Dezulovic became a little underwater game in Scratch. "
            "Luka Dezulovic is bringing its mermaid magic and peaceful aquarium to life in this 3D remake.",
            "Iz ideje Sabrine Dezulovic nastala je mala podvodna igra u Scratchu. "
            "Luka Dezulovic ovom 3D preradom oživljava njezinu sirensku čaroliju i mirni akvarij.",
            "Da un’idea di Sabrina Dezulovic è nato un piccolo gioco sottomarino in Scratch. "
            "Luka Dezulovic ne fa rivivere la magia e il tranquillo acquario in questo remake 3D."));
    loading_status = label(loading_ui, {135, 615}, {640, 40}, 20);
    loading_status->set_text(tr3("Please wait…", "Molimo pričekaj…", "Attendi…"));
    loading_bar = memnew(ProgressBar);
    loading_ui->add_child(loading_bar);
    loading_bar->set_position({135, 682});
    loading_bar->set_size({640, 10});
    loading_bar->set_show_percentage(false);
    loading_bar->add_theme_stylebox_override("background", box(Color(.04, .18, .25), 5));
    loading_bar->add_theme_stylebox_override("fill", box(cyan, 5));
    auto *copyright = label(loading_ui, {135, 720}, {640, 30}, 17);
    copyright->set_text(String::utf8("© 2026 Luka Dezulovic · Sabrinina podvodna bajka"));
}

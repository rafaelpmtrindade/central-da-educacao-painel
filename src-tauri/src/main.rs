// Sem janela de terminal no Windows
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    painel_do_dia_lib::run()
}

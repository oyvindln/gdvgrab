/*
 * Copyright (C) 2018 oln
 *
 * This program is free software; you can redistribute it and/or modify
 * it under the terms of the GNU General Public License as published by
 * the Free Software Foundation; either version 2 of the License, or
 * (at your option) any later version.
 *
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License
 * along with this program; if not, write to the Free Software Foundation,
 * Inc., 59 Temple Place - Suite 330, Boston, MA 02111-1307, USA.
 */

extern crate gtk;
extern crate subprocess;
extern crate config_file_handler;
#[macro_use]
extern crate serde_derive;

use gtk::prelude::*;
use gtk::{Button, Window, WindowType, TextView, CheckButton};
use std::path::PathBuf;
use std::rc::Rc;
use std::cell::RefCell;
use std::ops::{DerefMut, Deref};
use subprocess::{Exec, ExitStatus};

const REWIND_OPTION: &str = "-rewind";
const MAX_SIZE_OPTION: &str = "-s0";
const BUFFER_OPTIONS: [&str; 2] = ["-buffers", "1000"];
const FORMAT_OPTIONS: [&str; 2] = ["-f", "dv2"];
const PIPE_OPTION: &str = "-";
const PLAYER_COMMAND: &str = "ffplay";// -i pipe:";

const CONFIG_NAME: &str = "dvgrab";

#[derive(Default, Serialize, Deserialize)]
struct Config {
    last_path: PathBuf,
    guid: u64,
    buffer_size: i32,
    rewind_first: bool,
}

fn run_dvgrab_command(output: &PathBuf, rewind: bool) -> subprocess::Result<ExitStatus> {
    let cmd1 = if rewind {
        Exec::cmd("dvgrab").arg(REWIND_OPTION)
    } else {
        Exec::cmd("dvgrab")
    }.arg(MAX_SIZE_OPTION)
        .args(&BUFFER_OPTIONS)
        .args(&FORMAT_OPTIONS)
        .arg(output)
        .arg(PIPE_OPTION);


    let full_cmd = {
        cmd1 |
        Exec::cmd(PLAYER_COMMAND).
        //arg("-")
            arg("-i").
            arg("pipe:")
    };
    println!("Running: {:?}", full_cmd);
    full_cmd.join()
    //Exec::cmd(PLAYER_COMMAND).join()
}

fn show_message_dialog(message: &str, window: &Window, mtype: gtk::MessageType) {
    let dialog = gtk::MessageDialog::new(Some(window),
                                         gtk::DialogFlags::DESTROY_WITH_PARENT,
                                         mtype,
                                         gtk::ButtonsType::Close,
                                         message
    );
    dialog.run();
    dialog.destroy();
}

fn run_dvgrab(output: &PathBuf, rewind: bool, window: &Window) {
    let result = run_dvgrab_command(output, rewind);
    match result {
        Ok(status) => {
            if status.success() {
                //println!("Ran successfully!");
                show_message_dialog("Ran successfully!", window, gtk::MessageType::Info);
            } else {
                show_message_dialog(&format!("Ran unsuccessfully! Exit status was: {:?}", status),
                                    window,
                                    gtk::MessageType::Warning);
            }
        }
        Err(e) => {
            show_message_dialog(&format!("Failed to run dvgrab! Error: {}", e),
                                window,
                                gtk::MessageType::Error);
        }
    }
}

fn main() {

    let output_file_path: Rc<RefCell<Option<PathBuf>>> = Rc::new(RefCell::new(None));

    if gtk::init().is_err() {
        println!("Failed to initialize GTK.");
        return;
    }

    let config = Rc::new(RefCell::new({
        if let Ok(config_file) =
            config_file_handler::FileHandler::<Config>::new(CONFIG_NAME, false) {
                println!("Config path used: {:?}", config_file.path());
                config_file.read_file().unwrap_or_default()
            } else {
                Config::default()
            }
    }));

    let window = Rc::new(Window::new(WindowType::Toplevel));
    window.set_title("gdvgrab");

    window.connect_delete_event(|_, _| {
        gtk::main_quit();
        Inhibit(false)
    });

    let main_box = gtk::Box::new(gtk::Orientation::Vertical, 0);

    window.add(&main_box);

    let button = Button::new_with_label("Select output file");
    let start_button = Rc::new(Button::new_with_label("Start capture!"));
    start_button.set_sensitive(false);

    let file_name_view = Rc::new(TextView::new());
    let rewind_checkbox = Rc::new(CheckButton::new_with_label("Rewind before starting"));
    rewind_checkbox.set_active(config.borrow().rewind_first);

    let wc = window.clone();
    let fnw = file_name_view.clone();
    let ofp = output_file_path.clone();
    let sb = start_button.clone();
    let cf = config.clone();

    button.connect_clicked(move |_| {
        let output_dialog = gtk::FileChooserDialog::new::<Window>(
            Some("Output file"),
            Some(&wc),
            gtk::FileChooserAction::Save
        );

        output_dialog.set_filename(&cf.borrow().last_path);

        output_dialog.add_button("_Cancel", gtk::ResponseType::Cancel.into());
        output_dialog.add_button("_Save", gtk::ResponseType::Accept.into());
        // Notify if overwriting.
        output_dialog.set_do_overwrite_confirmation(true);

        let res = output_dialog.run();

        // let accept_response = gtk::ResponseType::Accept.into();

        if let Some(output) = output_dialog.get_filename() {
            if res == gtk::ResponseType::Accept {
                output_dialog.hide();
                fnw.get_buffer()
                    .expect("Fatal error! text view had no buffer for some reason!")
                    .set_text(output.to_str().unwrap_or("Filename was not valid unicode!"));
                let mut oref = ofp.borrow_mut();
                cf.borrow_mut().last_path = output.clone();
                *oref.deref_mut() = Some(output);
                sb.set_sensitive(true);
            } else {
                println!("Cancelled");
            }
        }
        output_dialog.destroy();
    });

    let ofp2 = output_file_path.clone();
    let sb2 = start_button.clone();
    let wc2 = window.clone();
    let rcb2 = rewind_checkbox.clone();

    start_button.connect_clicked(move |_| {
        let path_ref = ofp2.borrow();
        println!("Path ref: {:?}", path_ref);
        // Not sure how to avoid a copy here.
        let path = path_ref.clone().unwrap();
        sb2.set_sensitive(false);
        run_dvgrab(&path, rcb2.get_active(), &wc2);
        sb2.set_sensitive(true);
    });

    main_box.add(&*file_name_view);
    main_box.add(&button);
    main_box.add(&*rewind_checkbox);
    main_box.add(&*start_button);

    window.show_all();

    gtk::main();

    // Save config.
    if let Ok(config_file) = config_file_handler::FileHandler::<Config>::new(CONFIG_NAME, true) {
        config.borrow_mut().rewind_first = rewind_checkbox.deref().get_active();
        if let Err(e) = config_file.write_file(&config.borrow()) {
            println!("Failed to write config! {}", e);
        };
    }
}

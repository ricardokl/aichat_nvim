//use nvim_oxi::{Dictionary, Function, Object};
//
//mod ui;

//#[nvim_oxi::plugin]
//fn ui_rs() -> Dictionary {
//    let ui_select = Function::from_fn(|(title, items): (String, Vec<String>)| {
//        let _ = ui::ui_select(&title, items);
//    });
//
//    Dictionary::from_iter([("select", Object::from(ui_select))])
//}

#[nvim_oxi::plugin]
fn foo() -> i32 {
    42
}

#[derive(Clone, Debug)]
pub enum Popup {
    ConfirmDelete {
        record_display: String,
        record_id: String,
        endpoint: String,
    },
    Form {
        title: String,
        fields: Vec<FormField>,
        focused_field: usize,
        mode: FormMode,
        endpoint: String,
        id_field: String,
    },
    Help,
}

#[derive(Clone, Debug)]
pub struct FormField {
    pub label: String,
    pub value: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FormMode {
    Create,
    Edit,
}

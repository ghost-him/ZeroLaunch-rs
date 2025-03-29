use std::collections::HashSet;

use fontdb::Database;

/// 返回当前系统上安装的所有的字体
pub fn get_fonts() -> HashSet<String> {
    let mut result = HashSet::new();
    let mut db = Database::new();
    db.load_system_fonts();

    for face in db.faces() {
        let has_chinese = face
            .families
            .iter()
            .any(|(family, language)| language.primary_language() == "Chinese");

        if has_chinese {
            // 找到支持中文的字体名称
            if let Some((chinese_name, _)) = face
                .families
                .iter()
                .find(|(_, language)| language.primary_language() == "Chinese")
            {
                result.insert(chinese_name.clone());
            }
        } else {
            // 如果没有中文支持，使用默认名称（第一个家族名称）
            if !face.families.is_empty() {
                result.insert(face.families[0].0.clone());
            }
        }
    }
    result
}

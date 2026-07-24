use super::*;

#[tauri::command]
pub fn export_manuscript_markdown(
    input: ExportManuscriptInput,
    state: State<AppState>,
) -> Result<(), AppError> {
    with_active_connection(&state, |connection| {
        export_manuscript_markdown_in_connection(connection, &input.path)
    })
}

#[tauri::command]
pub fn export_manuscript_docx(
    input: ExportManuscriptInput,
    state: State<AppState>,
) -> Result<(), AppError> {
    with_active_connection(&state, |connection| {
        export_manuscript_docx_in_connection(connection, &input.path)
    })
}

#[tauri::command]
pub fn get_export_profile(state: State<AppState>) -> Result<ExportProfileSnapshot, AppError> {
    with_active_connection(&state, |connection| read_export_profile(connection))
}

#[tauri::command]
pub fn update_export_profile(
    input: UpdateExportProfileInput,
    state: State<AppState>,
) -> Result<ExportProfileSnapshot, AppError> {
    let profile = ExportProfileSnapshot {
        include_novel_title: input.include_novel_title,
        include_area_titles: input.include_area_titles,
        include_chapter_titles: input.include_chapter_titles,
        include_scene_titles: input.include_scene_titles,
        include_scene_text: input.include_scene_text,
    };
    validate_export_profile(&profile)?;
    with_active_connection(&state, |connection| {
        write_export_profile(connection, &profile)?;
        Ok(profile)
    })
}

pub(crate) fn validate_export_path(path: &str) -> Result<PathBuf, AppError> {
    let path = path.trim();
    if path.is_empty() {
        return Err(AppError::new(
            "invalid_export_path",
            "Bitte wähle einen Exportpfad aus.",
        ));
    }

    Ok(PathBuf::from(path))
}

pub(crate) fn validate_export_profile(profile: &ExportProfileSnapshot) -> Result<(), AppError> {
    if !profile.include_novel_title
        && !profile.include_area_titles
        && !profile.include_chapter_titles
        && !profile.include_scene_titles
        && !profile.include_scene_text
    {
        return Err(AppError::new(
            "empty_export_profile",
            "Wähle mindestens einen Inhalt für den Export aus.",
        ));
    }
    Ok(())
}

pub(crate) fn active_project_id(connection: &Connection) -> Result<String, AppError> {
    connection
        .query_row(
            "SELECT id FROM project WHERE deleted_at IS NULL LIMIT 1",
            [],
            |row| row.get(0),
        )
        .optional()?
        .ok_or_else(|| {
            AppError::new(
                "no_active_project",
                "Das Projekt enthält keinen aktiven Projektdatensatz.",
            )
        })
}

pub(crate) fn read_export_profile(
    connection: &Connection,
) -> Result<ExportProfileSnapshot, AppError> {
    let project_id = active_project_id(connection)?;
    let profile = connection.query_row(
        "SELECT include_novel_title, include_area_titles, include_chapter_titles, include_scene_titles, include_scene_text FROM export_profile WHERE project_id = ?1 AND deleted_at IS NULL LIMIT 1",
        params![project_id],
        |row| Ok(ExportProfileSnapshot { include_novel_title: row.get(0)?, include_area_titles: row.get(1)?, include_chapter_titles: row.get(2)?, include_scene_titles: row.get(3)?, include_scene_text: row.get(4)? }),
    ).optional()?;
    if let Some(profile) = profile {
        return Ok(profile);
    }
    let profile = ExportProfileSnapshot::default();
    write_export_profile(connection, &profile)?;
    Ok(profile)
}

pub(crate) fn write_export_profile(
    connection: &Connection,
    profile: &ExportProfileSnapshot,
) -> Result<(), AppError> {
    validate_export_profile(profile)?;
    let project_id = active_project_id(connection)?;
    let now = timestamp();
    let updated = connection.execute(
        "UPDATE export_profile SET include_novel_title = ?1, include_area_titles = ?2, include_chapter_titles = ?3, include_scene_titles = ?4, include_scene_text = ?5, updated_at = ?6 WHERE project_id = ?7 AND deleted_at IS NULL",
        params![profile.include_novel_title, profile.include_area_titles, profile.include_chapter_titles, profile.include_scene_titles, profile.include_scene_text, now, project_id],
    )?;
    if updated == 0 {
        connection.execute(
            "INSERT INTO export_profile (id, project_id, include_novel_title, include_area_titles, include_chapter_titles, include_scene_titles, include_scene_text, created_at, updated_at, deleted_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?8, NULL)",
            params![Uuid::new_v4().to_string(), project_id, profile.include_novel_title, profile.include_area_titles, profile.include_chapter_titles, profile.include_scene_titles, profile.include_scene_text, now],
        )?;
    }
    Ok(())
}

pub(crate) fn export_heading_level(profile: &ExportProfileSnapshot, level: usize) -> usize {
    let enabled = [
        profile.include_novel_title,
        profile.include_area_titles,
        profile.include_chapter_titles,
        profile.include_scene_titles,
    ];
    enabled[..=level]
        .iter()
        .filter(|enabled| **enabled)
        .count()
        .max(1)
}

pub(crate) fn scene_is_exported(scene: &SceneSnapshot, profile: &ExportProfileSnapshot) -> bool {
    profile.include_scene_titles || (profile.include_scene_text && !scene.text.trim().is_empty())
}

pub(crate) fn chapter_is_exported(
    chapter: &ChapterSnapshot,
    profile: &ExportProfileSnapshot,
) -> bool {
    profile.include_chapter_titles
        || chapter
            .scenes
            .iter()
            .any(|scene| scene_is_exported(scene, profile))
}

pub(crate) fn area_is_exported(area: &AreaSnapshot, profile: &ExportProfileSnapshot) -> bool {
    profile.include_area_titles
        || area
            .chapters
            .iter()
            .any(|chapter| chapter_is_exported(chapter, profile))
}

pub(crate) fn build_markdown_export(
    tree: &ManuscriptTree,
    profile: &ExportProfileSnapshot,
) -> String {
    let mut markdown = String::new();

    if profile.include_novel_title {
        markdown.push_str(&format!("# {}\n\n", tree.novel.title.trim()));
    }

    for area in &tree.areas {
        if !area_is_exported(area, profile) {
            continue;
        }
        if profile.include_area_titles {
            markdown.push_str(&format!(
                "{} {}\n\n",
                "#".repeat(export_heading_level(profile, 1)),
                area.title.trim()
            ));
        }

        for chapter in &area.chapters {
            if !chapter_is_exported(chapter, profile) {
                continue;
            }
            if profile.include_chapter_titles {
                markdown.push_str(&format!(
                    "{} {}\n\n",
                    "#".repeat(export_heading_level(profile, 2)),
                    chapter.title.trim()
                ));
            }

            for scene in &chapter.scenes {
                if !scene_is_exported(scene, profile) {
                    continue;
                }
                if profile.include_scene_titles {
                    markdown.push_str(&format!(
                        "{} {}\n\n",
                        "#".repeat(export_heading_level(profile, 3)),
                        scene.title.trim()
                    ));
                }
                if !profile.include_scene_text {
                    continue;
                }
                let text = scene.text.trim();
                if text.is_empty() {
                    continue;
                }
                markdown.push_str(text);
                markdown.push_str("\n\n");
            }
        }
    }

    if markdown.trim().is_empty() {
        String::new()
    } else {
        markdown.trim_end().to_string() + "\n"
    }
}

pub(crate) fn export_manuscript_markdown_in_connection(
    connection: &Connection,
    path: &str,
) -> Result<(), AppError> {
    let path = validate_export_path(path)?;
    let tree = read_manuscript_tree(connection)?;
    let profile = read_export_profile(connection)?;
    validate_export_profile(&profile)?;
    fs::write(path, build_markdown_export(&tree, &profile))?;

    Ok(())
}

pub(crate) fn export_manuscript_docx_in_connection(
    connection: &Connection,
    path: &str,
) -> Result<(), AppError> {
    let path = validate_export_path(path)?;
    let tree = read_manuscript_tree(connection)?;
    let profile = read_export_profile(connection)?;
    validate_export_profile(&profile)?;
    fs::write(path, build_docx_export(&tree, &profile)?)?;

    Ok(())
}

pub(crate) fn build_docx_export(
    tree: &ManuscriptTree,
    profile: &ExportProfileSnapshot,
) -> Result<Vec<u8>, AppError> {
    let cursor = Cursor::new(Vec::new());
    let mut zip = ZipWriter::new(cursor);
    let stored = FileOptions::default().compression_method(CompressionMethod::Stored);
    let deflated = FileOptions::default().compression_method(CompressionMethod::Deflated);

    zip.start_file("[Content_Types].xml", stored)?;
    zip.write_all(
        br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">
  <Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>
  <Default Extension="xml" ContentType="application/xml"/>
  <Override PartName="/word/document.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml"/>
  <Override PartName="/word/styles.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.styles+xml"/>
</Types>"#,
    )?;

    zip.add_directory("_rels/", stored)?;
    zip.start_file("_rels/.rels", stored)?;
    zip.write_all(
        br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="word/document.xml"/>
</Relationships>"#,
    )?;

    zip.add_directory("word/", stored)?;
    zip.start_file("word/document.xml", deflated)?;
    zip.write_all(build_docx_document_xml(tree, profile).as_bytes())?;

    zip.add_directory("word/_rels/", stored)?;
    zip.start_file("word/_rels/document.xml.rels", stored)?;
    zip.write_all(
        br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/styles" Target="styles.xml"/>
</Relationships>"#,
    )?;

    zip.start_file("word/styles.xml", deflated)?;
    zip.write_all(
        br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:styles xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
  <w:style w:type="paragraph" w:default="1" w:styleId="Normal">
    <w:name w:val="Normal"/>
    <w:qFormat/>
  </w:style>
  <w:style w:type="paragraph" w:styleId="Heading1">
    <w:name w:val="heading 1"/>
    <w:basedOn w:val="Normal"/>
    <w:next w:val="Normal"/>
    <w:qFormat/>
    <w:pPr><w:outlineLvl w:val="0"/></w:pPr>
    <w:rPr><w:b/><w:sz w:val="32"/></w:rPr>
  </w:style>
  <w:style w:type="paragraph" w:styleId="Heading2">
    <w:name w:val="heading 2"/>
    <w:basedOn w:val="Normal"/>
    <w:next w:val="Normal"/>
    <w:qFormat/>
    <w:pPr><w:outlineLvl w:val="1"/></w:pPr>
    <w:rPr><w:b/><w:sz w:val="26"/></w:rPr>
  </w:style>
  <w:style w:type="paragraph" w:styleId="Heading3">
    <w:name w:val="heading 3"/><w:basedOn w:val="Normal"/><w:next w:val="Normal"/><w:qFormat/>
    <w:pPr><w:outlineLvl w:val="2"/></w:pPr><w:rPr><w:b/><w:sz w:val="22"/></w:rPr>
  </w:style>
  <w:style w:type="paragraph" w:styleId="Heading4">
    <w:name w:val="heading 4"/><w:basedOn w:val="Normal"/><w:next w:val="Normal"/><w:qFormat/>
    <w:pPr><w:outlineLvl w:val="3"/></w:pPr><w:rPr><w:b/><w:sz w:val="20"/></w:rPr>
  </w:style>
</w:styles>"#,
    )?;

    let cursor = zip.finish()?;
    Ok(cursor.into_inner())
}

pub(crate) fn build_docx_document_xml(
    tree: &ManuscriptTree,
    profile: &ExportProfileSnapshot,
) -> String {
    let mut body = String::new();

    if profile.include_novel_title {
        body.push_str(&docx_paragraph(Some("Heading1"), tree.novel.title.trim()));
    }

    for area in &tree.areas {
        if !area_is_exported(area, profile) {
            continue;
        }
        if profile.include_area_titles {
            let style = format!("Heading{}", export_heading_level(profile, 1));
            body.push_str(&docx_paragraph(Some(&style), area.title.trim()));
        }

        for chapter in &area.chapters {
            if !chapter_is_exported(chapter, profile) {
                continue;
            }
            if profile.include_chapter_titles {
                let style = format!("Heading{}", export_heading_level(profile, 2));
                body.push_str(&docx_paragraph(Some(&style), chapter.title.trim()));
            }

            for scene in &chapter.scenes {
                if !scene_is_exported(scene, profile) {
                    continue;
                }
                if profile.include_scene_titles {
                    let style = format!("Heading{}", export_heading_level(profile, 3));
                    body.push_str(&docx_paragraph(Some(&style), scene.title.trim()));
                }
                if !profile.include_scene_text {
                    continue;
                }
                for paragraph in split_export_paragraphs(&scene.text) {
                    body.push_str(&docx_paragraph(None, &paragraph));
                }
            }
        }
    }

    format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
  <w:body>
    {body}
    <w:sectPr>
      <w:pgSz w:w="11906" w:h="16838"/>
      <w:pgMar w:top="1440" w:right="1440" w:bottom="1440" w:left="1440" w:header="708" w:footer="708" w:gutter="0"/>
    </w:sectPr>
  </w:body>
</w:document>"#
    )
}

pub(crate) fn split_export_paragraphs(text: &str) -> Vec<String> {
    text.replace("\r\n", "\n")
        .replace('\r', "\n")
        .split("\n\n")
        .map(str::trim)
        .filter(|paragraph| !paragraph.is_empty())
        .map(ToString::to_string)
        .collect()
}

pub(crate) fn docx_paragraph(style: Option<&str>, text: &str) -> String {
    let style_xml = style
        .map(|style| format!(r#"<w:pPr><w:pStyle w:val="{style}"/></w:pPr>"#))
        .unwrap_or_default();
    let runs = text
        .split('\n')
        .enumerate()
        .map(|(index, line)| {
            let prefix = if index == 0 { "" } else { "<w:br/>" };
            format!(
                r#"{prefix}<w:t xml:space="preserve">{}</w:t>"#,
                xml_escape(line)
            )
        })
        .collect::<Vec<_>>()
        .join("");

    format!("<w:p>{style_xml}<w:r>{runs}</w:r></w:p>")
}

pub(crate) fn xml_escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

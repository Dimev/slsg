use relative_path::{RelativePath, RelativePathBuf};

pub(crate) trait DoubleFileExt {
    fn has_double_ext(&self, ext: &str) -> bool;
    fn without_double_ext(&self) -> Option<RelativePathBuf>;
}

impl<T: AsRef<RelativePath>> DoubleFileExt for T {
    fn has_double_ext(&self, ext: &str) -> bool {
        // no file name, stop
        if self
            .as_ref()
            .file_name()
            .map(|x| [".", ".."].contains(&x))
            .unwrap_or(true)
        {
            return false;
        }

        let mut splits = self.as_ref().as_str().rsplit(".");
        splits.next();
        let second_ext = splits.next();
        second_ext.map(|x| x == ext).unwrap_or(false)
    }

    fn without_double_ext(&self) -> Option<RelativePathBuf> {
        // no file name, stop
        if self
            .as_ref()
            .file_name()
            .map(|x| [".", ".."].contains(&x))
            .unwrap_or(true)
        {
            return None;
        }

        let mut splits = self.as_ref().as_str().rsplitn(3, ".");
        let first_ext = splits.next()?;
        let _ = splits.next()?;
        let root = splits.next()?;

        Some(RelativePathBuf::from(format!("{root}.{first_ext}")))
    }
}

pub(crate) trait HtmlToIndex {
    fn html_to_index(&self) -> Option<RelativePathBuf>;
}

impl<T: AsRef<RelativePath>> HtmlToIndex for T {
    fn html_to_index(&self) -> Option<RelativePathBuf> {
        let ext = self
            .as_ref()
            .extension()
            .filter(|x| ["htm", "html"].contains(x))?;
        let file_stem = self.as_ref().file_stem()?;

        if file_stem == "index" {
            // already done, don't do anything
            None
        } else {
            // else, build the new string
            Some(
                self.as_ref()
                    .parent()?
                    .join(file_stem)
                    .join("index.htm")
                    .with_extension(ext),
            )
        }
    }
}

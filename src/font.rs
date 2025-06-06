use std::{
    borrow::Cow,
    cell::{Cell, Ref, RefCell},
    collections::{BTreeMap, BTreeSet},
    io::Cursor,
};

use hb_subset::Blob;
use html5ever::{
    Attribute, QualName, expanded_name,
    interface::{ElementFlags, NodeOrText, QuirksMode, TreeSink},
    local_name, ns, parse_document,
    tendril::{StrTendril, TendrilSink},
};
use mlua::{ErrorContext, ExternalResult, Result};

/// Subset a font
pub(crate) fn subset_font(file: &[u8], chars: &BTreeSet<char>) -> Result<Vec<u8>> {
    let font = Blob::from_bytes(file)
        .into_lua_err()
        .context("Failed to read font bytes")?;
    let subsetted = hb_subset::subset(&font, chars.iter().map(|x| *x))
        .into_lua_err()
        .context("Failed to subset font")?;
    Ok(subsetted)
}

// TODO: compression?: https://github.com/odemiral/woff2sfnt-sfnt2woff/blob/master/index.js

/// Get the characters from a html file
pub(crate) fn chars_from_html(html: &str) -> Result<BTreeSet<char>> {
    // make the sink
    let sink = Sink {
        charset: RefCell::new(BTreeSet::new()),
        next_id: Cell::new(0),
        names: RefCell::new(BTreeMap::new()),
    };

    // again cursed because html5ever expects this as a stream
    let mut stream = Cursor::new(html);
    let out = parse_document(sink, Default::default())
        .from_utf8()
        .read_from(&mut stream)
        .into_lua_err()
        .context("Failed to parse html")?;

    // and get out the characters we found
    Ok(out.charset.into_inner())
}

/// Sink for a character set
struct Sink {
    charset: RefCell<BTreeSet<char>>,
    next_id: Cell<usize>,
    names: RefCell<BTreeMap<usize, QualName>>,
}

// annoying as we have to traverse the tree using this
impl TreeSink for Sink {
    type Handle = usize;
    type Output = Self;
    type ElemName<'b> = Ref<'b, QualName>;

    // we care, as we can get text here
    fn append(&self, _: &usize, child: NodeOrText<usize>) {
        // this is the one we care about, as we might have text here
        match child {
            NodeOrText::AppendText(tendril) => {
                // text! push
                self.charset.borrow_mut().extend(tendril.chars());
            }
            _ => (),
        }
    }

    fn append_based_on_parent_node(&self, _: &usize, _: &usize, child: NodeOrText<usize>) {
        // this is the one we care about, as we might have text here
        match child {
            NodeOrText::AppendText(tendril) => {
                // text! push
                self.charset.borrow_mut().extend(tendril.chars());
            }
            _ => (),
        }
    }

    fn append_before_sibling(&self, _: &usize, node: NodeOrText<usize>) {
        // this is the one we care about, as we might have text here
        match node {
            NodeOrText::AppendText(tendril) => {
                // text! push
                self.charset.borrow_mut().extend(tendril.chars());
            }
            _ => (),
        }
    }

    // needed for html5ever
    fn finish(self) -> Self::Output {
        self
    }

    fn get_document(&self) -> usize {
        0
    }

    fn elem_name<'b>(&'b self, target: &'b usize) -> Self::ElemName<'b> {
        // get it from the stored elements
        Ref::map(self.names.borrow(), |x| &x[target])
    }

    fn create_element(&self, name: QualName, _: Vec<Attribute>, _: ElementFlags) -> usize {
        // increment
        self.next_id.set(self.next_id.get() + 1);

        // insert
        self.names
            .borrow_mut()
            .insert(self.next_id.get(), name.into());

        // put in the right spot
        self.next_id.get()
    }

    fn create_comment(&self, _: StrTendril) -> usize {
        // won't be rendered, just pretend we made an element
        self.next_id.replace(self.next_id.get() + 1)
    }

    fn create_pi(&self, _: StrTendril, _: StrTendril) -> usize {
        // won't be rendered, just pretend we made an element
        self.next_id.replace(self.next_id.get() + 1)
    }

    fn get_template_contents(&self, target: &usize) -> usize {
        if let Some(expanded_name!(html "template")) =
            self.names.borrow().get(target).map(|x| x.expanded())
        {
            target + 1
        } else {
            panic!("Not a template element");
        }
    }

    fn same_node(&self, x: &usize, y: &usize) -> bool {
        x == y
    }

    // we don't care what happens here
    fn parse_error(&self, _: Cow<'static, str>) {}
    fn reparent_children(&self, _: &usize, _: &usize) {}
    fn remove_from_parent(&self, _: &usize) {}
    fn add_attrs_if_missing(&self, _: &usize, _: Vec<Attribute>) {}
    fn set_quirks_mode(&self, _: QuirksMode) {}

    fn append_doctype_to_document(&self, _: StrTendril, _: StrTendril, _: StrTendril) {}
}

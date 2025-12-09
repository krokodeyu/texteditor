use std::fs;
use std::path::Path;
use std::collections::HashMap;

use crate::{
    error::{AppError, AppResult},
    commands::xml_command::XmlCommand,
};

// 易读性强，之后若要加入其他u32字段，可以转换为强类型以免混淆。
pub type NodeId = u32;

#[derive(Debug, Clone)]
pub struct XmlNode {
    id: NodeId,
    id_attr: String,
    name: String,
    parent: Option<NodeId>,
    attributes: HashMap<String, String>,
    // text与children字段互斥，需要在相关函数中进行判别。
    text: Option<String>,
    children: Vec<NodeId>,
}

impl XmlNode {
    pub fn id(&self) -> NodeId {
        self.id
    }

    pub fn id_attr(&self) -> &str {
        &self.id_attr
    }

    pub fn _name(&self) -> &str {
        &self.name
    }

    pub fn _parent(&self) -> Option<NodeId> {
        self.parent
    }

    pub fn _attributes(&self) -> &HashMap<String, String> {
        &self.attributes
    }

    pub fn children(&self) -> &[NodeId] {
        &self.children
    }

    pub fn _text(&self) -> Option<&str> {
        self.text.as_deref()
    }
}

/// 一棵被删除的子树（所有节点都从 XmlEditor 中移除了）
/// nodes 里放的是所有节点，root_id 是这棵树的根。
#[derive(Debug, Clone)]
pub struct DeletedSubtree {
    pub root_id: NodeId,
    pub nodes: Vec<XmlNode>,
}

#[derive(Debug, Clone)]
pub struct DeletedNodeToken {
    pub parent_id: NodeId,
    pub index: usize,
    pub subtree: DeletedSubtree,
}

/// 先做一个最简单的 XML 编辑器骨架：只保存原始字符串。
/// 之后再把 raw_text 换成 DOM 之类的结构。
#[derive(Default)]
pub struct XmlEditor {
    next_id: NodeId,
    root: NodeId,
    nodes: HashMap<NodeId, XmlNode>,

    // id 属性 -> NodeId 的映射，用于命令定位
    id_index: HashMap<String, NodeId>,

    modified: bool,
    logging: bool,

    // Undo/Redo操作用栈实现。
    undo_stack: Vec<Box<dyn XmlCommand>>,
    redo_stack: Vec<Box<dyn XmlCommand>>,
}

impl XmlEditor {
    pub fn new() -> Self { Self::default() }

    pub fn new_with_log(with_log: bool) -> Self {
        let mut nodes = HashMap::new();
        let mut id_index = HashMap::new();

        let root_id: NodeId = 1;

        let mut attrs = HashMap::new();
        if with_log {
            attrs.insert("log".into(), "true".into());
        }

        let root_node = XmlNode {
            id: root_id,
            id_attr: "root".into(),
            name: "root".into(),
            parent: None,
            attributes: attrs,
            children: Vec::new(),
            text: None,
        };

        nodes.insert(root_id, root_node);
        id_index.insert("root".into(), root_id);

        XmlEditor {
            next_id: root_id + 1,
            root: root_id,
            nodes,
            id_index,
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            modified: true,       // 新建缓冲区：已修改
            logging: with_log,    // 和 log 属性保持一致
        }
    }

    pub fn exec_xml(&mut self, mut cmd: Box<dyn XmlCommand>) -> AppResult<()> {
        cmd.execute(self)?;
        self.undo_stack.push(cmd);
        self.redo_stack.clear();
        Ok(())
    }

    pub fn undo(&mut self) -> AppResult<()> {
        if let Some(mut cmd) = self.undo_stack.pop() {
            cmd.undo(self)?;
            self.redo_stack.push(cmd);
            self.modified = true;
            Ok(())
        } else {
            Err(AppError::InvalidArgs("nothing to undo".into()))
        }
    }

    pub fn redo(&mut self) -> AppResult<()> {
        if let Some(mut cmd) = self.redo_stack.pop() {
            cmd.execute(self)?;
            self.undo_stack.push(cmd);
            self.modified = true;
            Ok(())
        } else {
            Err(AppError::InvalidArgs("nothing to redo".into()))
        }
    }

    pub fn load_from(&mut self, content: &str) -> AppResult<()> {
        let new_self = XmlEditor::from_string(content)?;
        *self = new_self;
        Ok(())
    }

    pub fn save_to(&mut self, p: impl AsRef<Path>) -> AppResult<()> {
        let text = self.to_string()?;
        fs::write(p.as_ref(), text)?; 
        self.modified = false;
        Ok(())
    }

    pub fn is_modified(&self) -> bool {
        self.modified
    }

    pub fn set_modified(&mut self, modified: bool) {
        self.modified = modified;
    }

    pub fn logging_enabled(&self) -> bool {
        self.logging
    }

    pub fn set_logging(&mut self, logging: bool) {
        self.logging = logging;
    }

    /// 先简单返回整段文本，后面再做带缩进/节点范围的 show。
    pub fn show_all(&self) -> AppResult<String> {
        self.to_string()
    }

    pub fn append_child(
        &mut self,
        tag_name: &String, 
        child_id: &String, 
        parent_id: &String, 
        text: Option<&String>
    ) -> AppResult<()>{
        // 找到父节点
        let parent_id: NodeId = self.find_by_attr_id(parent_id)?;
        // 借用问题：现在对parent进行不可变借用，需要及时终止借用值的生命周期。
        {
            let parent: &XmlNode = self.get_node(parent_id)?;
            if parent.text.is_some() {
                Err(AppError::InvalidArgs("父节点存在文本，无法插入".into()))?
            }
        }
        if self.has_attr_id(child_id) {
            Err(AppError::InvalidArgs(format!("id已存在：{}", child_id.clone()).into()))?
        }
        let child_text = match text {
            Some(t) => Some(t.clone()),
            None => None,
        };
        let new_id = self.alloc_id();
        let child: XmlNode = XmlNode {
            id: new_id,
            attributes: HashMap::new(),
            id_attr: child_id.clone(),
            name: tag_name.clone(),
            parent: Some(parent_id),
            text: child_text,
            children: Vec::new(),
        };

        // 登记子节点。
        self.nodes.insert(new_id, child);
        self.id_index.insert(child_id.clone(), new_id);

        let mut_parent = self.get_node_mut(parent_id)?;
        mut_parent.children.push(new_id);

        self.modified = true;
        
        Ok(())
    }

    pub fn delete_node(&mut self, attr_id: &String ) -> AppResult<DeletedNodeToken>{
        let id = self.find_by_attr_id(attr_id)?;
        let token = self.delete_node_by_id(id)?;
        self.modified = true;
        Ok(token)
    }

    pub fn restore_node(&mut self, token: DeletedNodeToken) -> AppResult<()> {
        let DeletedNodeToken {
            parent_id,
            index,
            subtree,
        } = token;

        let root_id = subtree.root_id;

        // 1. nodes / id_index 中恢复整棵子树
        self.restore_subtree(subtree)?;

        // 2. 把 root 节点重新挂回父节点 children
        {
            let parent = self.get_node_mut(parent_id)?;
            parent.children.insert(index, root_id);
        }

        self.modified = true;
        Ok(())
    }

    fn delete_node_by_id(&mut self, id: NodeId) -> AppResult<DeletedNodeToken>{
        if id == self.root {
            return Err(AppError::InvalidArgs("不能删除根元素".into()));
        }

        // 找 parent
        let parent_id = self.nodes[&id].parent.unwrap();

        // 找在 parent 中的下标
        let index = self.nodes[&parent_id]
            .children
            .iter()
            .position(|&c| c == id)
            .unwrap();

        // 从父节点删除 child
        self.nodes.get_mut(&parent_id).unwrap().children.remove(index);

        // 删除子树
        let subtree = self.remove_subtree(id)?;

        Ok(DeletedNodeToken {
            parent_id,
            index,
            subtree,
        })
    }

    /// 要求：父节点的 children 已经提前删除 child_id。
    fn remove_subtree(&mut self, root_id: NodeId) -> AppResult<DeletedSubtree> {
        let mut stack = Vec::new();
        let mut collected = Vec::new();

        stack.push(root_id);

        while let Some(id) = stack.pop() {
            // 从 nodes 中拿出这个节点
            let node = self.nodes.remove(&id).ok_or_else(|| {
                AppError::InternalError(format!("remove_subtree: node {} missing", id))
            })?;

            // 删除 id_attr 映射
            self.id_index.remove(node.id_attr());

            // 把所有子节点 id 入栈，递归删除
            for &child_id in node.children().iter() {
                stack.push(child_id);
            }

            collected.push(node);
        }

        Ok(DeletedSubtree {
            root_id,
            nodes: collected,
        })
    }

    fn restore_subtree(&mut self, subtree: DeletedSubtree) -> AppResult<()> {
        for node in subtree.nodes {
            let id = node.id();
            if self.nodes.contains_key(&id) {
                return Err(AppError::InternalError(
                    format!("restore_subtree: node id {} already exists", id),
                ));
            }

            // 恢复 id_index 映射
            self.id_index
                .insert(node.id_attr().to_string(), id);

            // 恢复节点本身
            self.nodes.insert(id, node);
        }

        Ok(())
    }

    fn find_by_attr_id(&self, attr_id: &str) -> AppResult<NodeId> {
        self.id_index
            .get(attr_id)
            .copied()
            .ok_or_else(|| AppError::InvalidArgs(format!("目标元素不存在: {}", attr_id)))
    }

    fn has_attr_id(&self, attr_id: &str) -> bool {
        self.id_index.contains_key(attr_id)
    }

    /// 辅助函数：从str解析一个DOM Tree。
    fn from_string(src: &str) -> AppResult<Self> {
        // 初始化一个空的 XmlEditor
        let mut ed = Self::default();

        // 1. 去掉前导空白
        let mut s = src.trim_start();

        // 2. 解析 XML 声明
        if !s.starts_with("<?xml") {
            return Err(AppError::InvalidArgs(
                "xml declaration missing or malformed".into(),
            ));
        }
        let decl_end = s
            .find("?>")
            .ok_or_else(|| AppError::InvalidArgs("unterminated xml declaration".into()))?;
        s = &s[decl_end + 2..]; // 跳过 "?>"

        // 3. 去掉声明后的空白，开始解析根元素
        let s = s.trim_start();
        let mut pos = 0usize;

        let root_id = ed.parse_element(s, &mut pos, None)?;

        // 4. 检查后面是否只有空白
        if s[pos..].trim().len() != 0 {
            return Err(AppError::InvalidArgs(
                "extra content found after root element".into(),
            ));
        }

        ed.root = root_id;

        // 5. 从根的属性里解析 logging 状态（如果 log="true"）
        if let Some(root_node) = ed.nodes.get(&root_id) {
            if let Some(v) = root_node.attributes.get("log") {
                if v == "true" {
                    ed.logging = true;
                }
            }
        }

        Ok(ed)
    }

    /// 辅助函数：解析一个XML元素
    fn parse_element(&mut self, s: &str, pos: &mut usize, parent: Option<NodeId>) -> AppResult<NodeId> {
        Self::skip_ws(s, pos);

        // 必须是 '<'
        Self::expect_char(s, pos, '<')?;

        // 防止 "</xxx>" 被错误当成开始标签
        if Self::starts_with(s, *pos, "/") {
            return Err(AppError::InvalidArgs(
                "unexpected closing tag while parsing element".into(),
            ));
        }

        // 标签名
        let name = Self::parse_name(s, pos)?;

        // 属性
        let attrs = Self::parse_attributes(s, pos)?;

        Self::skip_ws(s, pos);
        Self::expect_char(s, pos, '>')?;

        // 现在进入内容区域：可能是文本、子元素或空

        Self::skip_ws(s, pos);

        // 情况一：空元素 <tag ...></tag>
        if Self::starts_with(s, *pos, "</") {
            *pos += 2; // 跳过 "</"
            let end_name = Self::parse_name(s, pos)?;
            if end_name != name {
                return Err(AppError::InvalidArgs(format!(
                    "mismatched closing tag: expected </{}>, found </{}>",
                    name, end_name
                )));
            }
            Self::skip_ws(s, pos);
            Self::expect_char(s, pos, '>')?;

            let id = self.new_node(&name, attrs, None, parent)?;
            Ok(id)
        }
        // 情况二：子元素（以 '<' 开头且不是 </）
        else if Self::starts_with(s, *pos, "<") {
            let id = self.new_node(&name, attrs, None, parent)?;

            loop {
                Self::skip_ws(s, pos);
                if Self::starts_with(s, *pos, "</") {
                    // 读到当前元素的结束标签
                    *pos += 2;
                    let end_name = Self::parse_name(s, pos)?;
                    if end_name != name {
                        return Err(AppError::InvalidArgs(format!(
                            "mismatched closing tag: expected </{}>, found </{}>",
                            name, end_name
                        )));
                    }
                    Self::skip_ws(s, pos);
                    Self::expect_char(s, pos, '>')?;
                    break;
                } else {
                    // 子元素
                    let child_id = self.parse_element(s, pos, Some(id))?;
                    self.get_node_mut(id)?.children.push(child_id);
                }
            }

            Ok(id)
        }
        // 情况三：文本内容，直到遇到 "</"
        else {
            let text = Self::parse_text_until_lt(s, pos);

            if !Self::starts_with(s, *pos, "</") {
                return Err(AppError::InvalidArgs(
                    "expected closing tag after text content".into(),
                ));
            }
            *pos += 2;
            let end_name = Self::parse_name(s, pos)?;
            if end_name != name {
                return Err(AppError::InvalidArgs(format!(
                    "mismatched closing tag: expected </{}>, found </{}>",
                    name, end_name
                )));
            }
            Self::skip_ws(s, pos);
            Self::expect_char(s, pos, '>')?;

            let id = self.new_node(&name, attrs, Some(text), parent)?;
            Ok(id)
        }
    }

    ///创建新结点
    fn new_node(
        &mut self,
        name: &str,
        mut attributes: HashMap<String, String>,
        text: Option<String>,
        parent: Option<NodeId>,
    ) -> AppResult<NodeId> {
        // 每个元素必须有唯一 id 属性（字符串 id_attr）
        let id_attr = attributes
            .remove("id")
            .ok_or_else(|| AppError::InvalidArgs("xml element missing id attribute".into()))?;

        if self.id_index.contains_key(&id_attr) {
            return Err(AppError::InvalidArgs(format!(
                "duplicate xml id attribute: {}",
                id_attr
            )));
        }

        let id = self.alloc_id();

        let node = XmlNode {
            id,
            id_attr: id_attr.clone(),
            name: name.to_string(),
            parent,
            attributes,
            children: Vec::new(),
            text,
        };

        self.nodes.insert(id, node);
        self.id_index.insert(id_attr, id);

        Ok(id)
    }

    /// 按每次+1分配内部id，后续可以修改为别的逻辑。
    fn alloc_id(&mut self) -> NodeId {
        let id = self.next_id;
        self.next_id += 1;
        id
    }

    fn get_node(&self, id: NodeId) -> AppResult<&XmlNode> {
        self.nodes.get(&id).ok_or_else(|| AppError::InvalidArgs("xml node id not found".into()))
    }

    fn get_node_mut(&mut self, id: NodeId) -> AppResult<&mut XmlNode> {
        self.nodes
            .get_mut(&id)
            .ok_or_else(|| AppError::InvalidArgs("xml node id not found".into()))
    }

    fn skip_ws(s: &str, pos: &mut usize) {
        while let Some(c) = s[*pos..].chars().next() {
            if c.is_whitespace() {
                *pos += c.len_utf8();
            } else {
                break;
            }
        }
    }

    fn expect_char(s: &str, pos: &mut usize, ch: char) -> AppResult<()> {
        if let Some(c) = s[*pos..].chars().next() {
            if c == ch {
                *pos += c.len_utf8();
                Ok(())
            } else {
                Err(AppError::InvalidArgs(format!(
                    "expected '{}', found '{}'",
                    ch, c
                )))
            }
        } else {
            Err(AppError::InvalidArgs(format!(
                "expected '{}', found end of input",
                ch
            )))
        }
    }

    fn starts_with(s: &str, pos: usize, pat: &str) -> bool {
        s[pos..].starts_with(pat)
    }

    fn parse_name(s: &str, pos: &mut usize) -> AppResult<String> {
        let mut name = String::new();
        for c in s[*pos..].chars() {
            if c.is_alphanumeric() || c == '_' || c == '-' {
                name.push(c);
                *pos += c.len_utf8();
            } else {
                break;
            }
        }

        if name.is_empty() {
            Err(AppError::InvalidArgs("expected tag name".into()))
        } else {
            Ok(name)
        }
    }

    fn parse_attributes(s: &str, pos: &mut usize) -> AppResult<HashMap<String, String>> {
        let mut attrs = HashMap::new();

        loop {
            Self::skip_ws(s, pos);
            // 下一个是 '>' 或 "</"，说明属性结束
            if let Some(c) = s[*pos..].chars().next() {
                if c == '>' || c == '/' {
                    break;
                }
            } else {
                break;
            }

            // 解析属性名
            let key = Self::parse_name(s, pos)?;
            Self::skip_ws(s, pos);
            Self::expect_char(s, pos, '=')?;
            Self::skip_ws(s, pos);
            Self::expect_char(s, pos, '"')?;

            // 解析属性值直到下一个 '"'
            let mut value = String::new();
            while let Some(c) = s[*pos..].chars().next() {
                *pos += c.len_utf8();
                if c == '"' {
                    break;
                } else {
                    value.push(c);
                }
            }

            attrs.insert(key, value);
        }

        Ok(attrs)
    }

    fn parse_text_until_lt(s: &str, pos: &mut usize) -> String {
        let mut text = String::new();
        while let Some(c) = s[*pos..].chars().next() {
            if c == '<' {
                break;
            } else {
                text.push(c);
                *pos += c.len_utf8();
            }
        }
        text
    }

    fn to_string(&self) -> AppResult<String> {
        let mut out = String::new();

        // 1. XML 声明
        out.push_str(r#"<?xml version="1.0" encoding="UTF-8"?>"#);
        out.push('\n');

        // 2. 根节点
        self.serialize_node(self.root, 0, &mut out)?;

        Ok(out)
    }

    fn serialize_node(
        &self,
        id: NodeId,
        indent: usize,
        out: &mut String,
    ) -> AppResult<()> {
        let node = self
            .nodes
            .get(&id)
            .ok_or_else(|| AppError::InvalidArgs("xml node id not found".into()))?;

        let indent_str = " ".repeat(indent);

        // 开始标签
        out.push_str(&indent_str);
        out.push('<');
        out.push_str(&node.name);

        // 先写 id 属性
        out.push_str(r#" id=""#);
        out.push_str(&node.id_attr);
        out.push('"');

        // 再写其他普通属性
        for (k, v) in &node.attributes {
            out.push(' ');
            out.push_str(k);
            out.push_str("=\"");
            out.push_str(v);
            out.push('"');
        }

        out.push('>');

        // 有子节点
        if !node.children.is_empty() {
            out.push('\n');
            for &child_id in &node.children {
                self.serialize_node(child_id, indent + 4, out)?;
                out.push('\n');
            }
            out.push_str(&indent_str);
            out.push_str("</");
            out.push_str(&node.name);
            out.push('>');
        }
        // 有文本
        else if let Some(text) = &node.text {
            out.push_str(text);
            out.push_str("</");
            out.push_str(&node.name);
            out.push('>');
        }
        // 既没有文本也没有子节点：空元素
        else {
            out.push_str("</");
            out.push_str(&node.name);
            out.push('>');
        }

        Ok(())
    }
}

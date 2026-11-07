//! Integration test for real skills and books in `resource/skills` and `resource/books`.

use std::sync::Arc;
use adapter::{SearchStore, XerjAdapter};
use domain::book::BookCatalog;
use domain::memory::{SearchMode, SearchRequest};
use domain::skill::SkillCatalog;
use xcat::AppConfig;
use xcat_mcp::amemory::{
    FindBooksRequest, FindSkillsRequest, LoadBookRequest, LoadSkillRequest,
};
use xcat_mcp::rmcp::handler::server::wrapper::Parameters;
use xcat_mcp::{AmemoryToolProvider, DocumentImportPolicy};

#[tokio::test]
#[allow(clippy::too_many_lines)]
async fn test_skills_and_books_catalog_and_mcp_tools() {
    let config = AppConfig::load(None).expect("Failed to load AppConfig");

    // 1. 验证技能库文件与元数据解析
    let skills_catalog = SkillCatalog::new(&config.storage.skills);
    let skills = skills_catalog.collect_entries().expect("collect_entries failed");
    println!("=== 1. 已加载的技能目录 (共 {} 个技能) ===", skills.len());
    for s in &skills {
        println!("  - [Skill] 名称: {:<28} 描述: {}", s.name, s.description);
        assert!(!s.name.is_empty(), "Skill name cannot be empty");
        assert!(!s.description.is_empty(), "Skill description cannot be empty");

        let entry = skills_catalog.load_skill(&s.name).expect("Failed to load skill");
        assert!(!entry.skill_markdown.is_empty(), "SKILL.md cannot be empty for {}", s.name);
    }
    assert!(skills.len() >= 7, "Expected at least 7 skills");

    // 2. 验证电子书/知识库文件与章节解析
    let books_catalog = BookCatalog::new(&config.storage.books);
    let books = books_catalog.collect_entries().expect("collect_entries failed");
    println!("=== 2. 已加载的电子书/知识库 (共 {} 本书) ===", books.len());
    for b in &books {
        println!("  - [Book] 书名: {:<20} 描述: {:<30} 章节文件数: {}", b.name, b.description, b.files.len());
        assert!(!b.name.is_empty(), "Book name cannot be empty");

        let entry = books_catalog.load_book(&b.name).expect("Failed to load book");
        assert!(!entry.book_markdown.is_empty(), "BOOK.md cannot be empty for {}", b.name);
    }
    assert!(books.len() >= 2, "Expected at least 2 books (buspro and voip)");

    // 3. 打开底层 XERJ 向量/词法引擎并检查索引文档数
    let adapter = Arc::new(XerjAdapter::open(config.to_xerj_config()).expect("open XerjAdapter"));
    let stats = adapter.stats().await.expect("adapter stats");
    println!("=== 3. 内嵌 XERJ 引擎索引状态 ===");
    println!("  - 技能索引分片数 (skills_docs): {}", stats.skills_docs);
    println!("  - 电子书索引分片数 (books_docs):  {}", stats.books_docs);
    println!("  - 记忆索引分片数 (memory_docs): {}", stats.memory_docs);
    assert!(stats.skills_docs > 0, "Skills index should contain documents");
    assert!(stats.books_docs > 0, "Books index should contain documents");

    // 4. 验证全部技能语义/全文混合检索
    println!("=== 4. 测试技能语义/全文混合检索 ===");
    let skill_queries = [
        ("LVGL 控件与页面设计", "lvgl-maker"),
        ("宋姓宝宝起名 五行属水", "baby-name"),
        ("函数注释 Doxygen brief param", "ccomments"),
        ("程序员技术冷笑话", "dev-joke"),
        ("wirtec 一方 C代码约束", "wirtec"),
        ("check-c-arch 架构扫描违规", "check-c-arch"),
        ("embedded-layer-architecture 架构分层", "embedded-layer-architecture"),
    ];
    for (query, expected_skill) in skill_queries {
        let hits = adapter
            .search(&SearchRequest {
                query: query.to_string(),
                limit: 5,
                snippet_chars: 120,
                mode: SearchMode::Hybrid,
                uri: None,
                scope: Some("skill://".to_string()),
                exclude_uris: Vec::new(),
                exclude_scopes: Vec::new(),
                tag_filters: Vec::new(),
            })
            .await
            .expect("search skill");
        assert!(!hits.is_empty(), "Query '{query}' should return hits");
        let any_matched = hits
            .iter()
            .any(|h| h.uri.as_deref().unwrap_or_default().contains(expected_skill));
        println!(
            "  Query: {:<32} -> 命中 {} 条 | 首条: {} (期望命中: {})",
            format!("\"{query}\""),
            hits.len(),
            hits[0].uri.as_deref().unwrap_or_default(),
            expected_skill
        );
        assert!(any_matched, "Expected skill '{expected_skill}' in hits for query '{query}'");
    }

    // 5. 验证 XERJ 混合检索电子书
    println!("=== 5. 测试电子书语义/全文混合检索 ===");
    let book_queries = [
        ("SIP 呼叫流程 从拨号到接听", "voip"),
        ("NAT 穿透 内网通信", "voip"),
        ("Buspro 智能控制系统", "buspro"),
        ("SDP 协商 媒体格式", "voip"),
        ("802.1X 认证 网络准入", "voip"),
    ];
    for (query, expected_book) in book_queries {
        let hits = adapter
            .search(&SearchRequest {
                query: query.to_string(),
                limit: 5,
                snippet_chars: 120,
                mode: SearchMode::Hybrid,
                uri: None,
                scope: Some("book://".to_string()),
                exclude_uris: Vec::new(),
                exclude_scopes: Vec::new(),
                tag_filters: Vec::new(),
            })
            .await
            .expect("search book");
        assert!(!hits.is_empty(), "Query '{query}' should return hits");
        let any_matched = hits
            .iter()
            .any(|h| h.uri.as_deref().unwrap_or_default().contains(expected_book));
        println!(
            "  Query: {:<26} -> 命中 {} 条 | 首条: {} (期望命中: {})",
            format!("\"{query}\""),
            hits.len(),
            hits[0].uri.as_deref().unwrap_or_default(),
            expected_book
        );
        assert!(any_matched, "Expected book '{expected_book}' in hits for query '{query}'");
    }

    // 6. 验证面向 Zed Agent 的 MCP 接口执行
    println!("=== 6. 测试 MCP 工具执行 ===");
    let provider = AmemoryToolProvider::new(
        adapter.clone(),
        skills_catalog,
        books_catalog,
        DocumentImportPolicy::default(),
    );

    // 6.1 list_skills
    let res = provider.list_skills().await.expect("list_skills tool failed");
    let text = res.content[0].as_text().expect("text content");
    assert!(text.text.contains("lvgl-maker"));
    assert!(text.text.contains("wirtec"));
    println!("  [OK] MCP list_skills: 成功列出技能摘要");

    // 6.2 find_skills
    let res = provider
        .find_skills(Parameters(FindSkillsRequest {
            query: "LVGL 页面".to_string(),
            limit: 3,
        }))
        .await
        .expect("find_skills tool failed");
    let text = res.content[0].as_text().expect("text content");
    assert!(text.text.contains("lvgl-maker"));
    println!("  [OK] MCP find_skills('LVGL 页面'): 成功匹配 lvgl-maker");

    // 6.3 load_skill
    let res = provider
        .load_skill(Parameters(LoadSkillRequest {
            name: "lvgl-maker".to_string(),
            resource: None,
        }))
        .await
        .expect("load_skill tool failed");
    let text = res.content[0].as_text().expect("text content");
    assert!(!text.text.is_empty());
    println!("  [OK] MCP load_skill('lvgl-maker'): 成功加载 SKILL.md ({} 字符)", text.text.len());

    // 6.4 list_books
    let res = provider.list_books().await.expect("list_books tool failed");
    let text = res.content[0].as_text().expect("text content");
    assert!(text.text.contains("voip"));
    assert!(text.text.contains("buspro"));
    println!("  [OK] MCP list_books: 成功列出 voip 与 buspro 书籍");

    // 6.5 find_books
    let res = provider
        .find_books(Parameters(FindBooksRequest {
            query: "SIP 注册流程 电话上线".to_string(),
            limit: 3,
        }))
        .await
        .expect("find_books tool failed");
    let text = res.content[0].as_text().expect("text content");
    assert!(text.text.contains("voip"));
    println!("  [OK] MCP find_books('SIP 注册流程'): 成功检索到 voip 章节");

    // 6.6 load_book
    let res = provider
        .load_book(Parameters(LoadBookRequest {
            name: "voip".to_string(),
            resource: Some("11.SIP注册流程-电话如何上线.md".to_string()),
        }))
        .await
        .expect("load_book tool failed");
    let text = res.content[0].as_text().expect("text content");
    assert!(!text.text.is_empty());
    println!("  [OK] MCP load_book('voip', '11.SIP注册流程-电话如何上线.md'): 成功读取正文 ({} 字符)", text.text.len());

    println!("=======================================================");
    println!("所有 7 个技能与 2 部电子书/知识库全部测试通过，功能 100% 正常！");
    println!("=======================================================");
}

//! 共生 / 寄生 / 群体感应 / 生物膜系统 —— 基于真实生态学与微生物学研究
//!
//! 论文来源：
//! - Lynn Margulis, 1967, "On the origin of mitosing cells" —— 内共生学说
//! - Bassler, 2001, "How bacteria talk to each other" —— 群体感应
//! - Miller & Bassler, 2001, "Quorum sensing in bacteria"
//! - Costerton et al., 1995, "Microbial biofilms" —— 生物膜定义
//! - Hall-Stoodley, Costerton & Stoodley, 2004, "Bacterial biofilms"
//! - Flemming & Wingender, 2010, "The biofilm matrix" —— EPS 组成
//! - Hutchinson, 1957, "Concluding remarks" —— n 维生态位超体积

use serde::{Deserialize, Serialize};

// ============================================================
// 1. 共生类型
// ============================================================

/// 共生类型分类 —— 基于 de Bary 1879 原始定义扩展
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum SymbiosisType {
    /// 互利共生 —— 双方获益（地衣、菌根）
    Mutualism,
    /// 偏利共生 —— 一方获益，一方无影响（附生植物）
    Commensalism,
    /// 寄生 —— 一方获益，一方受害（疟原虫-人）
    Parasitism,
    /// 偏害共生 —— 一方受害，一方无影响（青霉菌产青霉素）
    Amensalism,
    /// 中性共生 —— 双方无影响
    Neutralism,
    /// 竞争 —— 双方互害（资源争夺）
    Competition,
    /// 内共生 —— 共生体生活于细胞内（线粒体）
    Endosymbiosis,
    /// 外共生 —— 共生体生活于细胞外
    Ectosymbiosis,
    /// 专性共生 —— 必须依赖（管虫-硫氧化菌）
    Obligate,
    /// 兼性共生 —— 非必须依赖
    Facultative,
}

/// 共生体传播方式
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum Transmission {
    /// 垂直传播 —— 亲代→子代（蚜虫-Buchnera，1.5 亿年）
    Vertical,
    /// 水平传播 —— 个体间传播
    Horizontal,
    /// 环境获取 —— 从环境获取（豆科-根瘤菌）
    Environmental,
}

/// 共生关系描述
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SymbioticRelationship {
    /// 宿主
    pub host: String,
    /// 共生体
    pub symbiont: String,
    /// 关系类型
    pub relationship_type: SymbiosisType,
    /// 宿主获益度（-1.0 伤害 ~ 1.0 获益）
    pub benefit_host: f32,
    /// 共生体获益度（-1.0 ~ 1.0）
    pub benefit_symbiont: f32,
    /// 传播方式
    pub transmission: Transmission,
    /// 特异性 0.0-1.0（1.0 高度专一）
    pub specificity: f32,
    /// 依赖性 0.0-1.0（1.0 必须共生）
    pub dependency: f32,
}
// ============================================================
// 2. 真实共生关系数据库
// ============================================================

/// 互利共生数据库 —— 经典案例 13 条
pub fn mutualism_database() -> Vec<SymbioticRelationship> {
    vec![
        // 豆科植物 + 根瘤菌 Rhizobium —— 固氮 N2 → NH3
        SymbioticRelationship {
            host: "豆科植物 Leguminosae".to_string(),
            symbiont: "根瘤菌 Rhizobium".to_string(),
            relationship_type: SymbiosisType::Mutualism,
            benefit_host: 0.8,
            benefit_symbiont: 0.7,
            transmission: Transmission::Environmental,
            specificity: 0.85,
            dependency: 0.7,
        },
        // 珊瑚 + 虫黄藻 Zooxanthellae —— 光合产物 → 珊瑚，CO2 → 藻
        SymbioticRelationship {
            host: "造礁珊瑚 Scleractinia".to_string(),
            symbiont: "虫黄藻 Symbiodinium".to_string(),
            relationship_type: SymbiosisType::Mutualism,
            benefit_host: 0.9,
            benefit_symbiont: 0.6,
            transmission: Transmission::Environmental,
            specificity: 0.7,
            dependency: 0.9,
        },
        // 地衣 Lichen —— 真菌 + 绿藻/蓝藻，5700 万年前共生
        SymbioticRelationship {
            host: "真菌 Mycobiont (Ascomycota)".to_string(),
            symbiont: "绿藻/蓝藻 Photobiont".to_string(),
            relationship_type: SymbiosisType::Mutualism,
            benefit_host: 0.8,
            benefit_symbiont: 0.5,
            transmission: Transmission::Horizontal,
            specificity: 0.6,
            dependency: 0.8,
        },
        // 白蚁 + 肠道鞭毛虫 —— 纤维素消化
        SymbioticRelationship {
            host: "白蚁 Isoptera".to_string(),
            symbiont: "肠道鞭毛虫 Trichonympha".to_string(),
            relationship_type: SymbiosisType::Mutualism,
            benefit_host: 0.9,
            benefit_symbiont: 0.7,
            transmission: Transmission::Horizontal,
            specificity: 0.9,
            dependency: 0.9,
        },
        // 蚜虫 + Buchnera aphidicola —— 必需氨基酸，垂直传播 1.5 亿年
        SymbioticRelationship {
            host: "蚜虫 Aphididae".to_string(),
            symbiont: "Buchnera aphidicola".to_string(),
            relationship_type: SymbiosisType::Mutualism,
            benefit_host: 0.85,
            benefit_symbiont: 0.9,
            transmission: Transmission::Vertical,
            specificity: 1.0,
            dependency: 1.0,
        },
        // 夏威夷短尾乌贼 + 发光弧菌 Vibrio fischeri
        SymbioticRelationship {
            host: "夏威夷短尾乌贼 Euprymna scolopes".to_string(),
            symbiont: "发光弧菌 Vibrio fischeri".to_string(),
            relationship_type: SymbiosisType::Mutualism,
            benefit_host: 0.7,
            benefit_symbiont: 0.8,
            transmission: Transmission::Environmental,
            specificity: 0.95,
            dependency: 0.6,
        },
        // 清洁鱼 Labroides + 客户鱼 —— 清洁体表寄生虫
        SymbioticRelationship {
            host: "客户鱼 Client fish".to_string(),
            symbiont: "清洁鱼 Labroides dimidiatus".to_string(),
            relationship_type: SymbiosisType::Mutualism,
            benefit_host: 0.6,
            benefit_symbiont: 0.7,
            transmission: Transmission::Horizontal,
            specificity: 0.3,
            dependency: 0.2,
        },
        // 小丑鱼 + 海葵 —— 保护 + 食物残渣
        SymbioticRelationship {
            host: "海葵 Heteractis magnifica".to_string(),
            symbiont: "小丑鱼 Amphiprion ocellaris".to_string(),
            relationship_type: SymbiosisType::Mutualism,
            benefit_host: 0.4,
            benefit_symbiont: 0.8,
            transmission: Transmission::Horizontal,
            specificity: 0.7,
            dependency: 0.5,
        },
        // 管虫 Riftia + 硫氧化菌 —— 无口无肠，全靠共生
        SymbioticRelationship {
            host: "巨型管虫 Riftia pachyptila".to_string(),
            symbiont: "硫氧化菌 Sulfur-oxidizing bacteria".to_string(),
            relationship_type: SymbiosisType::Endosymbiosis,
            benefit_host: 1.0,
            benefit_symbiont: 0.8,
            transmission: Transmission::Environmental,
            specificity: 0.95,
            dependency: 1.0,
        },
        // 反刍动物 + 瘤胃微生物群 —— 纤维素 → 挥发酸
        SymbioticRelationship {
            host: "反刍动物 Ruminantia".to_string(),
            symbiont: "瘤胃微生物群 Rumen microbiota".to_string(),
            relationship_type: SymbiosisType::Mutualism,
            benefit_host: 0.9,
            benefit_symbiont: 0.7,
            transmission: Transmission::Horizontal,
            specificity: 0.5,
            dependency: 0.85,
        },
        // 切叶蚁 + Leucoagaricus 真菌 —— 农业，6000 万年
        SymbioticRelationship {
            host: "切叶蚁 Atta".to_string(),
            symbiont: "Leucoagaricus 真菌".to_string(),
            relationship_type: SymbiosisType::Mutualism,
            benefit_host: 0.8,
            benefit_symbiont: 0.85,
            transmission: Transmission::Vertical,
            specificity: 0.95,
            dependency: 0.9,
        },
        // 人类 + 肠道菌群 —— 合成维生素 K、B，免疫调节
        SymbioticRelationship {
            host: "人类 Homo sapiens".to_string(),
            symbiont: "肠道菌群 Gut microbiota".to_string(),
            relationship_type: SymbiosisType::Mutualism,
            benefit_host: 0.7,
            benefit_symbiont: 0.7,
            transmission: Transmission::Horizontal,
            specificity: 0.4,
            dependency: 0.5,
        },
        // 丛枝菌根 AMF —— Glomeromycota，4.5 亿年，80% 陆生植物
        SymbioticRelationship {
            host: "陆生植物 (80% 物种)".to_string(),
            symbiont: "丛枝菌根 AMF (Glomeromycota)".to_string(),
            relationship_type: SymbiosisType::Mutualism,
            benefit_host: 0.85,
            benefit_symbiont: 0.7,
            transmission: Transmission::Horizontal,
            specificity: 0.3,
            dependency: 0.75,
        },
        // 外生菌根 ECM —— 90% 植物根
        SymbioticRelationship {
            host: "森林树木 Pinaceae".to_string(),
            symbiont: "外生菌根 ECM (Basidiomycota)".to_string(),
            relationship_type: SymbiosisType::Mutualism,
            benefit_host: 0.8,
            benefit_symbiont: 0.75,
            transmission: Transmission::Horizontal,
            specificity: 0.6,
            dependency: 0.8,
        },
    ]
}
/// 寄生数据库 —— 经典案例 15 条
pub fn parasitism_database() -> Vec<SymbioticRelationship> {
    vec![
        // 疟原虫 Plasmodium + 蚊/人 —— 复杂生活史
        SymbioticRelationship {
            host: "人/按蚊 Anopheles".to_string(),
            symbiont: "疟原虫 Plasmodium falciparum".to_string(),
            relationship_type: SymbiosisType::Parasitism,
            benefit_host: -0.9,
            benefit_symbiont: 0.9,
            transmission: Transmission::Horizontal,
            specificity: 0.85,
            dependency: 1.0,
        },
        // 锥虫 Trypanosoma + 采采蝇/人 —— 昏睡病
        SymbioticRelationship {
            host: "人/采采蝇 Glossina".to_string(),
            symbiont: "锥虫 Trypanosoma brucei".to_string(),
            relationship_type: SymbiosisType::Parasitism,
            benefit_host: -0.85,
            benefit_symbiont: 0.85,
            transmission: Transmission::Horizontal,
            specificity: 0.8,
            dependency: 1.0,
        },
        // 利什曼原虫 Leishmania + 白蛉/人
        SymbioticRelationship {
            host: "人/白蛉 Phlebotomus".to_string(),
            symbiont: "利什曼原虫 Leishmania".to_string(),
            relationship_type: SymbiosisType::Parasitism,
            benefit_host: -0.8,
            benefit_symbiont: 0.85,
            transmission: Transmission::Horizontal,
            specificity: 0.75,
            dependency: 1.0,
        },
        // 弓形虫 Toxoplasma + 猫/啮齿 —— 行为操纵，老鼠不怕猫
        SymbioticRelationship {
            host: "啮齿类/猫 Felidae".to_string(),
            symbiont: "弓形虫 Toxoplasma gondii".to_string(),
            relationship_type: SymbiosisType::Parasitism,
            benefit_host: -0.5,
            benefit_symbiont: 0.9,
            transmission: Transmission::Horizontal,
            specificity: 0.7,
            dependency: 0.9,
        },
        // 血吸虫 Schistosoma + 钉螺/人 —— 2 亿人感染
        SymbioticRelationship {
            host: "人/钉螺 Oncomelania".to_string(),
            symbiont: "血吸虫 Schistosoma".to_string(),
            relationship_type: SymbiosisType::Parasitism,
            benefit_host: -0.8,
            benefit_symbiont: 0.85,
            transmission: Transmission::Horizontal,
            specificity: 0.85,
            dependency: 1.0,
        },
        // 绦虫 Taenia + 猪/牛/人
        SymbioticRelationship {
            host: "人/猪/牛".to_string(),
            symbiont: "绦虫 Taenia solium".to_string(),
            relationship_type: SymbiosisType::Parasitism,
            benefit_host: -0.6,
            benefit_symbiont: 0.8,
            transmission: Transmission::Horizontal,
            specificity: 0.7,
            dependency: 0.85,
        },
        // 蛔虫 Ascaris + 人 —— 8 亿人
        SymbioticRelationship {
            host: "人".to_string(),
            symbiont: "蛔虫 Ascaris lumbricoides".to_string(),
            relationship_type: SymbiosisType::Parasitism,
            benefit_host: -0.55,
            benefit_symbiont: 0.75,
            transmission: Transmission::Horizontal,
            specificity: 0.65,
            dependency: 0.8,
        },
        // 丝虫 Wuchereria + 蚊/人 —— 象皮肿
        SymbioticRelationship {
            host: "人/蚊".to_string(),
            symbiont: "丝虫 Wuchereria bancrofti".to_string(),
            relationship_type: SymbiosisType::Parasitism,
            benefit_host: -0.75,
            benefit_symbiont: 0.8,
            transmission: Transmission::Horizontal,
            specificity: 0.8,
            dependency: 0.95,
        },
        // 跳蚤 + 哺乳动物 —— 鼠疫 Yersinia pestis 媒介
        SymbioticRelationship {
            host: "哺乳动物".to_string(),
            symbiont: "跳蚤 Siphonaptera".to_string(),
            relationship_type: SymbiosisType::Parasitism,
            benefit_host: -0.5,
            benefit_symbiont: 0.7,
            transmission: Transmission::Horizontal,
            specificity: 0.4,
            dependency: 0.6,
        },
        // 蜱 + 哺乳动物 —— 莱姆病 Borrelia
        SymbioticRelationship {
            host: "哺乳动物".to_string(),
            symbiont: "蜱 Ixodes".to_string(),
            relationship_type: SymbiosisType::Parasitism,
            benefit_host: -0.6,
            benefit_symbiont: 0.7,
            transmission: Transmission::Horizontal,
            specificity: 0.5,
            dependency: 0.65,
        },
        // 杜鹃鸟 Brood parasite —— 巢寄生
        SymbioticRelationship {
            host: "其他雀形目鸟".to_string(),
            symbiont: "杜鹃 Cuculus".to_string(),
            relationship_type: SymbiosisType::Parasitism,
            benefit_host: -0.7,
            benefit_symbiont: 0.7,
            transmission: Transmission::Horizontal,
            specificity: 0.6,
            dependency: 0.7,
        },
        // 噬菌体 + 细菌 —— 病毒寄生细菌
        SymbioticRelationship {
            host: "细菌".to_string(),
            symbiont: "噬菌体 Phage".to_string(),
            relationship_type: SymbiosisType::Parasitism,
            benefit_host: -1.0,
            benefit_symbiont: 0.9,
            transmission: Transmission::Horizontal,
            specificity: 0.9,
            dependency: 1.0,
        },
        // 菟丝子 Cuscuta —— 全寄生植物
        SymbioticRelationship {
            host: "宿主植物".to_string(),
            symbiont: "菟丝子 Cuscuta".to_string(),
            relationship_type: SymbiosisType::Parasitism,
            benefit_host: -0.7,
            benefit_symbiont: 0.85,
            transmission: Transmission::Horizontal,
            specificity: 0.5,
            dependency: 1.0,
        },
        // 列当 Orobanchaceae —— 根寄生
        SymbioticRelationship {
            host: "宿主植物根".to_string(),
            symbiont: "列当 Orobanchaceae".to_string(),
            relationship_type: SymbiosisType::Parasitism,
            benefit_host: -0.65,
            benefit_symbiont: 0.85,
            transmission: Transmission::Horizontal,
            specificity: 0.7,
            dependency: 1.0,
        },
        // 吸虫 Trematode + 多宿主
        SymbioticRelationship {
            host: "螺/鱼/人".to_string(),
            symbiont: "吸虫 Trematode (Clonorchis)".to_string(),
            relationship_type: SymbiosisType::Parasitism,
            benefit_host: -0.7,
            benefit_symbiont: 0.85,
            transmission: Transmission::Horizontal,
            specificity: 0.75,
            dependency: 1.0,
        },
    ]
}
/// 内共生学说数据库 —— Lynn Margulis 1967
///
/// 真核细胞器起源于远古内共生事件：
/// - 线粒体：15-20 亿年前，α-变形菌入侵古真核细胞
/// - 叶绿体：10-15 亿年前，蓝藻被真核生物吞噬
/// - 次级内共生：原生动物吞噬藻类形成顶体
pub fn endosymbiosis_theory() -> Vec<SymbioticRelationship> {
    vec![
        // 线粒体起源于 α-变形菌 —— 15-20 亿年前
        SymbioticRelationship {
            host: "古真核细胞 (Asgard archaea)".to_string(),
            symbiont: "α-变形菌 (Rickettsiales)".to_string(),
            relationship_type: SymbiosisType::Endosymbiosis,
            benefit_host: 1.0,
            benefit_symbiont: 0.9,
            transmission: Transmission::Vertical,
            specificity: 1.0,
            dependency: 1.0,
        },
        // 叶绿体起源于蓝藻 —— 10-15 亿年前
        SymbioticRelationship {
            host: "早期真核生物".to_string(),
            symbiont: "蓝藻 Cyanobacteria".to_string(),
            relationship_type: SymbiosisType::Endosymbiosis,
            benefit_host: 1.0,
            benefit_symbiont: 0.9,
            transmission: Transmission::Vertical,
            specificity: 1.0,
            dependency: 1.0,
        },
        // 氢化酶体 Hydrogenosome —— 厌氧原生动物
        SymbioticRelationship {
            host: "厌氧原生动物 (Trichomonas)".to_string(),
            symbiont: "产氢细菌 (线粒体衍生物)".to_string(),
            relationship_type: SymbiosisType::Endosymbiosis,
            benefit_host: 0.85,
            benefit_symbiont: 0.7,
            transmission: Transmission::Vertical,
            specificity: 0.95,
            dependency: 0.95,
        },
        // 顶体 Apicoplast —— 疟原虫，次级内共生
        SymbioticRelationship {
            host: "疟原虫 Plasmodium".to_string(),
            symbiont: "红藻 (次级内共生)".to_string(),
            relationship_type: SymbiosisType::Endosymbiosis,
            benefit_host: 0.7,
            benefit_symbiont: 0.85,
            transmission: Transmission::Vertical,
            specificity: 0.95,
            dependency: 0.9,
        },
    ]
}

// ============================================================
// 3. 群体感应（Quorum Sensing, QS）
// ============================================================
// 来源：Bassler 2001, Miller & Bassler 2001
// Vibrio fischeri 经典模型：>10^10 cells/mL 触发发光

/// 群体感应信号分子类型
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum SignalMolecule {
    /// 酰基高丝氨酸内酯 AHL —— 革兰阴性，Vibrio fischeri
    AHL,
    /// 自诱导肽 AIP —— 革兰阳性，葡萄球菌
    AIP,
    /// 自诱导物-2 AI-2 —— 跨物种通讯，呋喃硼酸酯
    AI2,
    /// 扩散信号因子 DSF —— Xanthomonas
    DSF,
    /// 假单胞菌喹诺酮信号 PQS —— Pseudomonas
    Pseudomonas,
}

/// 群体感应系统 —— Bassler 2001
///
/// QS 行为示例：
/// - Vibrio fischeri：>10^10 cells/mL → 发光
/// - Pseudomonas aeruginosa：毒力因子、生物膜形成
/// - Agrobacterium：Ti 质粒接合转移
/// - Bacillus：感受态、孢子形成
/// - Streptomyces：抗生素生产
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuorumSensing {
    /// 信号分子类型
    pub signal: SignalMolecule,
    /// 合成酶（LuxI 类）
    pub synthase: String,
    /// 受体（LuxR 类）
    pub receptor: String,
    /// 激活阈值浓度 (nM)
    pub threshold_conc_nm: f32,
    /// 当前信号浓度 (nM)
    pub current_conc_nm: f32,
    /// 受调控基因
    pub regulated_genes: Vec<String>,
    /// 是否激活
    pub activated: bool,
}

impl QuorumSensing {
    /// 新建群体感应系统 —— 按信号分子类型配置默认合成酶/受体
    pub fn new(signal: SignalMolecule) -> Self {
        let (synthase, receptor, threshold) = match signal {
            SignalMolecule::AHL => ("LuxI".to_string(), "LuxR".to_string(), 10.0),
            SignalMolecule::AIP => ("AgrD".to_string(), "AgrC".to_string(), 50.0),
            SignalMolecule::AI2 => ("LuxS".to_string(), "LsrB".to_string(), 5.0),
            SignalMolecule::DSF => ("RpfF".to_string(), "RpfC".to_string(), 25.0),
            SignalMolecule::Pseudomonas => ("PqsA".to_string(), "PqsR".to_string(), 20.0),
        };
        let regulated_genes = match signal {
            SignalMolecule::AHL => vec![
                "luxCDABE".to_string(),
                "luxI".to_string(),
                "luxR".to_string(),
            ],
            SignalMolecule::AIP => vec![
                "agrBDCA".to_string(),
                "hld".to_string(),
                "spa".to_string(),
            ],
            SignalMolecule::AI2 => vec!["lsrACDBFG".to_string(), "lsrRK".to_string()],
            SignalMolecule::DSF => vec![
                "engA".to_string(),
                "rpfF".to_string(),
                "rpfC".to_string(),
            ],
            SignalMolecule::Pseudomonas => vec![
                "pqsABCDE".to_string(),
                "phnAB".to_string(),
                "pqhR".to_string(),
            ],
        };
        Self {
            signal,
            synthase,
            receptor,
            threshold_conc_nm: threshold,
            current_conc_nm: 0.0,
            regulated_genes,
            activated: false,
        }
    }

    /// 更新 QS 状态 —— 基于细胞密度与时间步长
    ///
    /// 信号动力学：
    /// - 合成速率 ∝ cell_density（约 1 nM/min at 10^9 cells/mL）
    /// - 降解半衰期 ~30 min（AHL 内酯酶水解）
    /// - 激活阈值含迟滞效应（已激活维持阈值降低至 70%）
    pub fn update(&mut self, cell_density: f32, dt_min: f32) {
        let synthesis_rate = cell_density * 0.01 * dt_min;
        let degradation_rate = self.current_conc_nm * 0.023 * dt_min;
        self.current_conc_nm += synthesis_rate - degradation_rate;
        if self.current_conc_nm < 0.0 {
            self.current_conc_nm = 0.0;
        }
        let activation_threshold = if self.activated {
            self.threshold_conc_nm * 0.7
        } else {
            self.threshold_conc_nm
        };
        self.activated = self.current_conc_nm >= activation_threshold;
    }

    /// 根据当前信号浓度反推细胞密度估计值
    ///
    /// 稳态假设：合成 = 降解 → density = current × 0.023 / 0.01
    pub fn cell_density_estimate(&self) -> f32 {
        self.current_conc_nm * 2.3
    }
}
// ============================================================
// 4. 生物膜（Biofilm）
// ============================================================
// 来源：Costerton 1995, Hall-Stoodley 2004

/// 生物膜发育阶段
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum BiofilmStage {
    /// 浮游态 —— 自由游动
    Planktonic,
    /// 初始附着 —— 可逆
    InitialAttachment,
    /// 不可逆附着
    IrreversibleAttachment,
    /// 微菌落
    Microcolony,
    /// 成熟期 I —— 塔状结构
    Maturation1,
    /// 成熟期 II —— 蘑菇状
    Maturation2,
    /// 分散 —— 释放浮游细胞
    Dispersion,
}

/// 胞外聚合物 EPS —— biofilm 基质（Flemming & Wingender 2010）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EPS {
    /// 多糖 40-95%
    pub polysaccharides_pct: f32,
    /// 蛋白质 1-60%
    pub proteins_pct: f32,
    /// 胞外 DNA 1-10%
    pub edna_pct: f32,
    /// 脂质 1-40%
    pub lipids_pct: f32,
    /// 含水量 ~97%
    pub water_content_pct: f32,
}

impl EPS {
    /// 默认 EPS 组成 —— Flemming & Wingender 2010
    pub fn new() -> Self {
        Self {
            polysaccharides_pct: 60.0,
            proteins_pct: 15.0,
            edna_pct: 5.0,
            lipids_pct: 5.0,
            water_content_pct: 97.0,
        }
    }
}

impl Default for EPS {
    fn default() -> Self {
        Self::new()
    }
}

/// 生物膜 —— Costerton 1995
///
/// 典型厚度 10-500 μm，抗生素抗性可提升 10-1000 倍
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Biofilm {
    /// 物种组成
    pub species: Vec<String>,
    /// 发育阶段
    pub stage: BiofilmStage,
    /// 厚度 (μm)，10-500 μm
    pub thickness_um: f32,
    /// 占据面积 (mm²)
    pub area_mm2: f32,
    /// 细胞数
    pub cell_count: u64,
    /// EPS 基质
    pub eps_matrix: EPS,
    /// 水通道数 —— 营养输送与代谢废物排出
    pub channels: u32,
    /// 抗生素抗性提升倍数（10-1000×）
    pub antibiotic_resistance_factor: f32,
}

impl Biofilm {
    /// 新建生物膜 —— 初始浮游态，少量游离细胞
    pub fn new(species: Vec<String>) -> Self {
        Self {
            species,
            stage: BiofilmStage::Planktonic,
            thickness_um: 0.0,
            area_mm2: 0.0,
            cell_count: 100,
            eps_matrix: EPS::new(),
            channels: 0,
            antibiotic_resistance_factor: 1.0,
        }
    }

    /// 单步推进 —— 受营养物浓度调控
    ///
    /// 阶段时长参考 Costerton 1995:
    /// - 初始附着: ~1 小时
    /// - 微菌落: ~6 小时
    /// - 成熟期: 数日-数周
    /// - 分散: 营养耗竭或群体密度过载触发
    pub fn step(&mut self, dt_min: f32, nutrients: f32) {
        let dt_h = dt_min / 60.0;

        // 细胞增长 —— 营养驱动的 logistic 增长
        let growth_rate = 0.5 * nutrients * dt_h;
        let next = (self.cell_count as f32) * (1.0 + growth_rate);
        self.cell_count = next.max(1.0) as u64;

        // 阶段推进
        match self.stage {
            BiofilmStage::Planktonic => {
                if self.cell_count > 500 {
                    self.stage = BiofilmStage::InitialAttachment;
                    self.area_mm2 = 0.001;
                }
            }
            BiofilmStage::InitialAttachment => {
                self.area_mm2 += 0.0005 * dt_h;
                if self.cell_count > 2000 {
                    self.stage = BiofilmStage::IrreversibleAttachment;
                    self.eps_matrix.polysaccharides_pct = 65.0;
                }
            }
            BiofilmStage::IrreversibleAttachment => {
                self.thickness_um += 0.5 * dt_h;
                if self.thickness_um > 5.0 {
                    self.stage = BiofilmStage::Microcolony;
                    self.antibiotic_resistance_factor = 10.0;
                }
            }
            BiofilmStage::Microcolony => {
                self.thickness_um += 1.0 * dt_h;
                self.channels = ((self.thickness_um as u32) / 10).max(1);
                if self.thickness_um > 50.0 {
                    self.stage = BiofilmStage::Maturation1;
                    self.antibiotic_resistance_factor = 100.0;
                }
            }
            BiofilmStage::Maturation1 => {
                self.thickness_um += 2.0 * dt_h;
                let target_channels = ((self.thickness_um as u32) / 20).max(self.channels);
                self.channels = target_channels;
                if self.thickness_um > 200.0 {
                    self.stage = BiofilmStage::Maturation2;
                    self.antibiotic_resistance_factor = 500.0;
                }
            }
            BiofilmStage::Maturation2 => {
                self.thickness_um += (1.0 * dt_h).min(50.0);
                if nutrients < 0.1 || self.thickness_um > 450.0 {
                    self.stage = BiofilmStage::Dispersion;
                }
            }
            BiofilmStage::Dispersion => {
                self.disperse();
            }
        }
    }

    /// 分散 —— 释放浮游细胞，回归初始状态
    ///
    /// 通常由营养耗竭、群体密度过载、剪切力触发
    pub fn disperse(&mut self) -> u64 {
        let released = self.cell_count / 10;
        self.cell_count -= released;
        if self.cell_count < 100 {
            self.stage = BiofilmStage::Planktonic;
            self.thickness_um = 0.0;
            self.area_mm2 = 0.0;
            self.channels = 0;
            self.antibiotic_resistance_factor = 1.0;
        }
        released
    }

    /// 抗生素穿透因子 —— biofilm 物理屏障 + 代谢耐受
    ///
    /// 渗透率 0.0（完全阻挡）到 1.0（完全穿透）
    /// biofilm 可使抗性提升 10-1000×，对应穿透率 0.001-0.1
    pub fn penetrance_factor(&self, antibiotic_conc_ug_ml: f32) -> f32 {
        let eps_barrier = 1.0 / self.antibiotic_resistance_factor;
        let thickness_barrier = (-self.thickness_um / 100.0).exp();
        let conc_factor = (antibiotic_conc_ug_ml / 10.0).min(1.0);
        (eps_barrier * thickness_barrier * conc_factor).clamp(0.0, 1.0)
    }

    /// 返回该生物膜内的群体感应系统列表 —— 按物种配置信号分子
    pub fn quorum_signals(&self) -> Vec<QuorumSensing> {
        let mut qs_list = Vec::new();
        for sp in &self.species {
            let mut q = if sp.contains("Pseudomonas") || sp.contains("pseudomonas") {
                QuorumSensing::new(SignalMolecule::Pseudomonas)
            } else if sp.contains("Staphylococcus") || sp.contains("葡萄球菌") {
                QuorumSensing::new(SignalMolecule::AIP)
            } else if sp.contains("Vibrio") || sp.contains("弧菌") {
                QuorumSensing::new(SignalMolecule::AHL)
            } else if sp.contains("Xanthomonas") {
                QuorumSensing::new(SignalMolecule::DSF)
            } else {
                QuorumSensing::new(SignalMolecule::AI2)
            };
            q.current_conc_nm = ((self.cell_count as f32) / 1e8).min(100.0);
            q.activated = q.current_conc_nm >= q.threshold_conc_nm;
            qs_list.push(q);
        }
        qs_list
    }
}
// ============================================================
// 5. 微生物互作网络
// ============================================================

/// 微生物互作类型
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum MicrobialInteraction {
    /// 交叉喂养 —— A 代谢产物给 B
    CrossFeeding,
    /// 营养竞争
    Competition,
    /// 抑制 —— 抗生素产生
    Amensalism,
    /// 捕食 —— Bdellovibrio 捕食细菌
    Predation,
    /// 寄生 —— 噬菌体
    Parasitism,
    /// 互养 —— H2 转移，甲烷生产
    Syntrophy,
    /// 偏利
    Commensalism,
    /// 促进
    Facilitation,
}

/// 微生物互作网络边
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MicrobialInteractionEdge {
    /// 来源物种
    pub from: String,
    /// 目标物种
    pub to: String,
    /// 互作类型
    pub interaction: MicrobialInteraction,
    /// 强度 -1.0 ~ 1.0（负向有害，正向有益）
    pub strength: f32,
    /// 涉及代谢物
    pub metabolites: Vec<String>,
}

/// 典型微生物互作 —— 经典案例 8 条
pub fn typical_interactions() -> Vec<MicrobialInteractionEdge> {
    vec![
        // Bacteroides + Faecalibacterium —— 交叉喂养，乙酸 → 丁酸
        MicrobialInteractionEdge {
            from: "Bacteroides thetaiotaomicron".to_string(),
            to: "Faecalibacterium prausnitzii".to_string(),
            interaction: MicrobialInteraction::CrossFeeding,
            strength: 0.6,
            metabolites: vec!["乙酸 acetate".to_string(), "丁酸 butyrate".to_string()],
        },
        // 反硝化菌 + 固氮菌 —— 氮循环互养
        MicrobialInteractionEdge {
            from: "固氮菌 Azotobacter".to_string(),
            to: "反硝化菌 Pseudomonas denitrificans".to_string(),
            interaction: MicrobialInteraction::CrossFeeding,
            strength: 0.5,
            metabolites: vec!["NH4+".to_string(), "NO3-".to_string()],
        },
        // 硫酸盐还原菌 + 硫氧化菌 —— 互养 H2 转移
        MicrobialInteractionEdge {
            from: "硫酸盐还原菌 Desulfovibrio".to_string(),
            to: "硫氧化菌 Thiobacillus".to_string(),
            interaction: MicrobialInteraction::Syntrophy,
            strength: 0.7,
            metabolites: vec!["H2S".to_string(), "SO4^2-".to_string()],
        },
        // Bdellovibrio + 大肠杆菌 —— 捕食
        MicrobialInteractionEdge {
            from: "Bdellovibrio bacteriovorus".to_string(),
            to: "大肠杆菌 E. coli".to_string(),
            interaction: MicrobialInteraction::Predation,
            strength: -0.8,
            metabolites: vec![],
        },
        // 噬菌体 + 任何细菌 —— 寄生
        MicrobialInteractionEdge {
            from: "噬菌体 T4".to_string(),
            to: "大肠杆菌 E. coli".to_string(),
            interaction: MicrobialInteraction::Parasitism,
            strength: -0.95,
            metabolites: vec![],
        },
        // 乳酸菌抑制致病菌 —— pH 下降
        MicrobialInteractionEdge {
            from: "乳酸菌 Lactobacillus".to_string(),
            to: "致病菌 Salmonella".to_string(),
            interaction: MicrobialInteraction::Amensalism,
            strength: -0.6,
            metabolites: vec!["乳酸 lactic acid".to_string(), "pH<4.5".to_string()],
        },
        // 放线菌产抗生素
        MicrobialInteractionEdge {
            from: "放线菌 Streptomyces".to_string(),
            to: "革兰阳性菌".to_string(),
            interaction: MicrobialInteraction::Amensalism,
            strength: -0.7,
            metabolites: vec!["链霉素 streptomycin".to_string()],
        },
        // 双歧杆菌 + 大肠杆菌 —— 乙酸促进 / 低 pH 抑制致病
        MicrobialInteractionEdge {
            from: "双歧杆菌 Bifidobacterium".to_string(),
            to: "大肠杆菌 E. coli".to_string(),
            interaction: MicrobialInteraction::Facilitation,
            strength: 0.4,
            metabolites: vec!["乙酸 acetate".to_string(), "低 pH".to_string()],
        },
    ]
}

// ============================================================
// 6. 生态位理论
// ============================================================

/// 生态位 —— Hutchinson 1957 n-维超体积模型
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EcologicalNiche {
    /// 资源描述
    pub resource: String,
    /// pH 适应范围
    pub ph_range: (f32, f32),
    /// 温度范围 (°C)
    pub temp_range_c: (f32, f32),
    /// 氧气需求 0.0 厌氧 ~ 1.0 完全需氧
    pub oxygen: f32,
    /// 湿度 0.0-1.0
    pub moisture: f32,
    /// 宿主关联（None 表示自由生活）
    pub host_association: Option<String>,
}
// ============================================================
// 单元测试模块
// ============================================================

#[cfg(test)]
mod tests {
    use super::*;

    // ---------- SymbiosisType / Transmission 枚举 ----------

    #[test]
    fn test_symbiosis_type_variants_exist() {
        let _ = SymbiosisType::Mutualism;
        let _ = SymbiosisType::Commensalism;
        let _ = SymbiosisType::Parasitism;
        let _ = SymbiosisType::Amensalism;
        let _ = SymbiosisType::Neutralism;
        let _ = SymbiosisType::Competition;
        let _ = SymbiosisType::Endosymbiosis;
        let _ = SymbiosisType::Ectosymbiosis;
        let _ = SymbiosisType::Obligate;
        let _ = SymbiosisType::Facultative;
    }

    #[test]
    fn test_transmission_variants_exist() {
        let _ = Transmission::Vertical;
        let _ = Transmission::Horizontal;
        let _ = Transmission::Environmental;
    }

    // ---------- 数据库函数 ----------

    #[test]
    fn test_mutualism_database_count() {
        let v = mutualism_database();
        assert_eq!(v.len(), 14, "mutualism_database should have 14 entries");
        for r in &v {
            // all mutualism should benefit both (or be Endosymbiosis)
            assert!(
                r.relationship_type == SymbiosisType::Mutualism
                    || r.relationship_type == SymbiosisType::Endosymbiosis,
                "entry {} should be Mutualism/Endosymbiosis",
                r.symbiont
            );
            assert!(r.benefit_host >= 0.0, "host benefit should be >= 0");
            assert!(r.benefit_symbiont >= 0.0, "symbiont benefit should be >= 0");
        }
    }

    #[test]
    fn test_parasitism_database_count() {
        let v = parasitism_database();
        assert_eq!(v.len(), 15, "parasitism_database should have 15 entries");
        for r in &v {
            assert_eq!(r.relationship_type, SymbiosisType::Parasitism);
            // host is harmed in parasitism
            assert!(r.benefit_host < 0.0, "host should be harmed, got {}", r.benefit_host);
            // symbiont benefits
            assert!(r.benefit_symbiont > 0.0);
        }
    }

    #[test]
    fn test_endosymbiosis_theory_count() {
        let v = endosymbiosis_theory();
        assert_eq!(v.len(), 4);
        for r in &v {
            assert_eq!(r.relationship_type, SymbiosisType::Endosymbiosis);
            // vertical transmission (Margulis 内共生为垂直)
            assert_eq!(r.transmission, Transmission::Vertical);
            // high specificity and dependency
            assert!(r.specificity >= 0.9);
            assert!(r.dependency >= 0.9);
        }
    }

    #[test]
    fn test_mutualism_database_contains_rhizobium() {
        let v = mutualism_database();
        let found = v.iter().any(|r| r.symbiont.contains("Rhizobium"));
        assert!(found, "should contain Rhizobium mutualism");
    }

    #[test]
    fn test_mutualism_database_contains_buchnera_vertical() {
        let v = mutualism_database();
        let buchnera = v.iter().find(|r| r.symbiont.contains("Buchnera"));
        assert!(buchnera.is_some(), "Buchnera entry should exist");
        let b = buchnera.unwrap();
        assert_eq!(b.transmission, Transmission::Vertical);
        assert_eq!(b.specificity, 1.0);
        assert_eq!(b.dependency, 1.0);
    }

    #[test]
    fn test_typical_interactions_count() {
        let v = typical_interactions();
        assert_eq!(v.len(), 8);
        for e in &v {
            assert!(!e.from.is_empty());
            assert!(!e.to.is_empty());
            assert!(e.strength >= -1.0 && e.strength <= 1.0);
        }
    }

    #[test]
    fn test_typical_interactions_predation_negative_strength() {
        let v = typical_interactions();
        let pred = v.iter().find(|e| e.interaction == MicrobialInteraction::Predation);
        assert!(pred.is_some());
        assert!(pred.unwrap().strength < 0.0, "predation strength should be negative");
    }

    // ---------- QuorumSensing ----------

    #[test]
    fn test_quorum_sensing_new_ahl_config() {
        let qs = QuorumSensing::new(SignalMolecule::AHL);
        assert_eq!(qs.signal, SignalMolecule::AHL);
        assert_eq!(qs.synthase, "LuxI");
        assert_eq!(qs.receptor, "LuxR");
        assert!((qs.threshold_conc_nm - 10.0).abs() < 1e-6);
        assert_eq!(qs.current_conc_nm, 0.0);
        assert!(!qs.activated);
        assert!(qs.regulated_genes.contains(&"luxCDABE".to_string()));
    }

    #[test]
    fn test_quorum_sensing_new_aip_config() {
        let qs = QuorumSensing::new(SignalMolecule::AIP);
        assert_eq!(qs.synthase, "AgrD");
        assert_eq!(qs.receptor, "AgrC");
        assert!((qs.threshold_conc_nm - 50.0).abs() < 1e-6);
        assert!(qs.regulated_genes.contains(&"hld".to_string()));
    }

    #[test]
    fn test_quorum_sensing_new_ai2_config() {
        let qs = QuorumSensing::new(SignalMolecule::AI2);
        assert_eq!(qs.synthase, "LuxS");
        assert_eq!(qs.receptor, "LsrB");
        assert!((qs.threshold_conc_nm - 5.0).abs() < 1e-6);
        assert!(qs.regulated_genes.contains(&"lsrACDBFG".to_string()));
    }

    #[test]
    fn test_quorum_sensing_new_dsf_config() {
        let qs = QuorumSensing::new(SignalMolecule::DSF);
        assert_eq!(qs.synthase, "RpfF");
        assert_eq!(qs.receptor, "RpfC");
        assert!((qs.threshold_conc_nm - 25.0).abs() < 1e-6);
    }

    #[test]
    fn test_quorum_sensing_new_pseudomonas_config() {
        let qs = QuorumSensing::new(SignalMolecule::Pseudomonas);
        assert_eq!(qs.synthase, "PqsA");
        assert_eq!(qs.receptor, "PqsR");
        assert!((qs.threshold_conc_nm - 20.0).abs() < 1e-6);
        assert!(qs.regulated_genes.contains(&"pqsABCDE".to_string()));
    }

    #[test]
    fn test_quorum_sensing_update_activates_when_above_threshold() {
        let mut qs = QuorumSensing::new(SignalMolecule::AHL); // threshold = 10
        // high cell density, long dt -> concentration should rise above threshold
        qs.update(10000.0, 100.0);
        assert!(qs.activated, "should be activated, conc={}", qs.current_conc_nm);
        assert!(qs.current_conc_nm >= 10.0);
    }

    #[test]
    fn test_quorum_sensing_update_not_activated_below_threshold() {
        let mut qs = QuorumSensing::new(SignalMolecule::AHL);
        qs.update(1.0, 1.0);
        // synthesis = 1 * 0.01 * 1 = 0.01, far below 10
        assert!(!qs.activated);
    }

    #[test]
    fn test_quorum_sensing_update_hysteresis_stays_activated() {
        let mut qs = QuorumSensing::new(SignalMolecule::AHL); // threshold = 10
        // push above threshold to activate
        qs.current_conc_nm = 15.0;
        qs.update(0.0, 1.0);
        assert!(qs.activated, "should be activated at 15");
        // decay slightly but stay above 0.7 * 10 = 7
        // degradation = 15 * 0.023 * 1 = 0.345 -> new conc ~14.655, still > 7
        assert!(qs.activated, "hysteresis should keep activated");
    }

    #[test]
    fn test_quorum_sensing_update_zero_density_degrades_signal() {
        let mut qs = QuorumSensing::new(SignalMolecule::AHL);
        qs.current_conc_nm = 50.0;
        qs.update(0.0, 10.0);
        // synthesis = 0; degradation = 50 * 0.023 * 10 = 11.5 -> conc ~38.5
        assert!(qs.current_conc_nm < 50.0, "should decay, got {}", qs.current_conc_nm);
    }

    #[test]
    fn test_quorum_sensing_update_negative_conc_clamped_to_zero() {
        let mut qs = QuorumSensing::new(SignalMolecule::AHL);
        qs.current_conc_nm = 0.0;
        qs.update(0.0, 1000.0);
        // no synthesis, no degradation issue, should remain 0
        assert!((qs.current_conc_nm - 0.0).abs() < 1e-6);
        assert!(!qs.activated);
    }

    #[test]
    fn test_quorum_sensing_cell_density_estimate_formula() {
        let mut qs = QuorumSensing::new(SignalMolecule::AHL);
        qs.current_conc_nm = 10.0;
        // estimate = current * 2.3 = 23
        assert!((qs.cell_density_estimate() - 23.0).abs() < 1e-5);
    }

    // ---------- EPS ----------

    #[test]
    fn test_eps_new_default_composition() {
        let eps = EPS::new();
        assert!((eps.polysaccharides_pct - 60.0).abs() < 1e-6);
        assert!((eps.proteins_pct - 15.0).abs() < 1e-6);
        assert!((eps.edna_pct - 5.0).abs() < 1e-6);
        assert!((eps.lipids_pct - 5.0).abs() < 1e-6);
        assert!((eps.water_content_pct - 97.0).abs() < 1e-6);
    }

    #[test]
    fn test_eps_default_equals_new() {
        let d = EPS::default();
        let n = EPS::new();
        assert!((d.polysaccharides_pct - n.polysaccharides_pct).abs() < 1e-6);
        assert!((d.proteins_pct - n.proteins_pct).abs() < 1e-6);
        assert!((d.edna_pct - n.edna_pct).abs() < 1e-6);
        assert!((d.water_content_pct - n.water_content_pct).abs() < 1e-6);
    }

    // ---------- Biofilm ----------

    #[test]
    fn test_biofilm_new_initial_state() {
        let bf = Biofilm::new(vec!["E. coli".into()]);
        assert_eq!(bf.stage, BiofilmStage::Planktonic);
        assert_eq!(bf.cell_count, 100);
        assert!((bf.thickness_um - 0.0).abs() < 1e-6);
        assert!((bf.area_mm2 - 0.0).abs() < 1e-6);
        assert_eq!(bf.channels, 0);
        assert!((bf.antibiotic_resistance_factor - 1.0).abs() < 1e-6);
        assert_eq!(bf.species.len(), 1);
    }

    #[test]
    fn test_biofilm_step_grows_cell_count_with_nutrients() {
        let mut bf = Biofilm::new(vec!["E. coli".into()]);
        let before = bf.cell_count;
        bf.step(60.0, 1.0); // 1h, full nutrients
        assert!(bf.cell_count > before, "expected growth, before={} after={}", before, bf.cell_count);
    }

    #[test]
    fn test_biofilm_step_zero_nutrients_no_growth() {
        let mut bf = Biofilm::new(vec!["E. coli".into()]);
        let before = bf.cell_count;
        bf.step(60.0, 0.0);
        // growth_rate = 0.5 * 0 * 1 = 0 -> next = 100 * 1 = 100
        assert_eq!(bf.cell_count, before);
    }

    #[test]
    fn test_biofilm_step_transitions_to_initial_attachment() {
        let mut bf = Biofilm::new(vec!["E. coli".into()]);
        // force cell_count > 500 to trigger transition
        bf.cell_count = 600;
        bf.step(1.0, 1.0);
        assert_eq!(bf.stage, BiofilmStage::InitialAttachment);
        assert!((bf.area_mm2 - 0.001).abs() < 1e-6);
    }

    #[test]
    fn test_biofilm_step_irreversible_increases_thickness() {
        let mut bf = Biofilm::new(vec!["E. coli".into()]);
        bf.stage = BiofilmStage::IrreversibleAttachment;
        let before = bf.thickness_um;
        bf.step(60.0, 1.0); // 1h -> thickness += 0.5
        assert!(bf.thickness_um > before);
    }

    #[test]
    fn test_biofilm_disperse_releases_one_tenth() {
        let mut bf = Biofilm::new(vec!["E. coli".into()]);
        bf.cell_count = 1000;
        let released = bf.disperse();
        assert_eq!(released, 100);
        assert_eq!(bf.cell_count, 900);
    }

    #[test]
    fn test_biofilm_disperse_below_100_resets_to_planktonic() {
        let mut bf = Biofilm::new(vec!["E. coli".into()]);
        bf.cell_count = 50; // < 100
        bf.stage = BiofilmStage::Maturation1;
        bf.thickness_um = 100.0;
        bf.channels = 5;
        bf.antibiotic_resistance_factor = 100.0;
        let _ = bf.disperse();
        // 50 / 10 = 5 released, 45 left; 45 < 100 -> reset
        assert_eq!(bf.stage, BiofilmStage::Planktonic);
        assert!((bf.thickness_um - 0.0).abs() < 1e-6);
        assert_eq!(bf.channels, 0);
        assert!((bf.antibiotic_resistance_factor - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_biofilm_penetrance_factor_zero_thickness_zero_conc() {
        let bf = Biofilm::new(vec!["E. coli".into()]);
        // thickness=0, resistance=1, conc=0
        // eps_barrier = 1/1 = 1; thickness_barrier = exp(0)=1; conc_factor = min(0/10,1)=0
        // result = 1 * 1 * 0 = 0
        let p = bf.penetrance_factor(0.0);
        assert!((p - 0.0).abs() < 1e-6);
    }

    #[test]
    fn test_biofilm_penetrance_factor_decreases_with_thickness() {
        let mut bf_thick = Biofilm::new(vec!["E. coli".into()]);
        bf_thick.thickness_um = 200.0;
        bf_thick.antibiotic_resistance_factor = 100.0;
        let p_thick = bf_thick.penetrance_factor(100.0);

        let mut bf_thin = Biofilm::new(vec!["E. coli".into()]);
        bf_thin.thickness_um = 10.0;
        bf_thin.antibiotic_resistance_factor = 1.0;
        let p_thin = bf_thin.penetrance_factor(100.0);

        assert!(p_thick < p_thin, "thicker biofilm should have lower penetrance");
    }

    #[test]
    fn test_biofilm_penetrance_factor_clamped_to_one() {
        let bf = Biofilm::new(vec!["E. coli".into()]);
        // very high conc -> conc_factor = min(high/10, 1) = 1
        // but eps_barrier * thickness_barrier * 1, thickness=0 -> exp(0)=1
        // eps_barrier = 1/1 = 1 -> result = 1, clamp OK
        let p = bf.penetrance_factor(1000.0);
        assert!(p <= 1.0);
    }

    // ---------- Biofilm::quorum_signals ----------

    #[test]
    fn test_biofilm_quorum_signals_pseudomonas_detected() {
        let bf = Biofilm::new(vec!["Pseudomonas aeruginosa".into()]);
        let qs = bf.quorum_signals();
        assert_eq!(qs.len(), 1);
        assert_eq!(qs[0].signal, SignalMolecule::Pseudomonas);
        assert_eq!(qs[0].synthase, "PqsA");
    }

    #[test]
    fn test_biofilm_quorum_signals_staph_aip_detected() {
        let bf = Biofilm::new(vec!["Staphylococcus aureus".into()]);
        let qs = bf.quorum_signals();
        assert_eq!(qs.len(), 1);
        assert_eq!(qs[0].signal, SignalMolecule::AIP);
    }

    #[test]
    fn test_biofilm_quorum_signals_vibrio_ahl_detected() {
        let bf = Biofilm::new(vec!["Vibrio cholerae".into()]);
        let qs = bf.quorum_signals();
        assert_eq!(qs.len(), 1);
        assert_eq!(qs[0].signal, SignalMolecule::AHL);
    }

    #[test]
    fn test_biofilm_quorum_signals_default_ai2_for_unknown() {
        let bf = Biofilm::new(vec!["E. coli".into()]);
        let qs = bf.quorum_signals();
        assert_eq!(qs.len(), 1);
        assert_eq!(qs[0].signal, SignalMolecule::AI2);
    }

    #[test]
    fn test_biofilm_quorum_signals_activated_when_dense() {
        let mut bf = Biofilm::new(vec!["Vibrio fischeri".into()]);
        // threshold AHL = 10; current = cell_count / 1e8, capped at 100
        // need cell_count >= 10 * 1e8 = 1e9
        bf.cell_count = 2_000_000_000;
        let qs = bf.quorum_signals();
        assert!(qs[0].activated, "should be activated at high density");
        assert!(qs[0].current_conc_nm >= qs[0].threshold_conc_nm);
    }

    #[test]
    fn test_biofilm_quorum_signals_multiple_species() {
        let bf = Biofilm::new(vec![
            "Pseudomonas aeruginosa".into(),
            "Staphylococcus aureus".into(),
            "Vibrio cholerae".into(),
        ]);
        let qs = bf.quorum_signals();
        assert_eq!(qs.len(), 3);
        assert_eq!(qs[0].signal, SignalMolecule::Pseudomonas);
        assert_eq!(qs[1].signal, SignalMolecule::AIP);
        assert_eq!(qs[2].signal, SignalMolecule::AHL);
    }

    // ---------- MicrobialInteraction 枚举 ----------

    #[test]
    fn test_microbial_interaction_variants_exist() {
        let _ = MicrobialInteraction::CrossFeeding;
        let _ = MicrobialInteraction::Competition;
        let _ = MicrobialInteraction::Amensalism;
        let _ = MicrobialInteraction::Predation;
        let _ = MicrobialInteraction::Parasitism;
        let _ = MicrobialInteraction::Syntrophy;
        let _ = MicrobialInteraction::Commensalism;
        let _ = MicrobialInteraction::Facilitation;
    }

    #[test]
    fn test_signal_molecule_variants_exist() {
        let _ = SignalMolecule::AHL;
        let _ = SignalMolecule::AIP;
        let _ = SignalMolecule::AI2;
        let _ = SignalMolecule::DSF;
        let _ = SignalMolecule::Pseudomonas;
    }

    #[test]
    fn test_biofilm_stage_variants_exist() {
        let _ = BiofilmStage::Planktonic;
        let _ = BiofilmStage::InitialAttachment;
        let _ = BiofilmStage::IrreversibleAttachment;
        let _ = BiofilmStage::Microcolony;
        let _ = BiofilmStage::Maturation1;
        let _ = BiofilmStage::Maturation2;
        let _ = BiofilmStage::Dispersion;
    }
}
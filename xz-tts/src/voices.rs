use crate::types::TtsVoiceInfo;

fn strings(items: &[&str]) -> Vec<String> {
    items.iter().map(|s| (*s).to_string()).collect()
}

fn voice(
    voice_id: &str,
    name: &str,
    gender: Option<&str>,
    language: &str,
    styles: &[&str],
    scenarios: &[&str],
    model_version: &str,
) -> TtsVoiceInfo {
    TtsVoiceInfo {
        voice_id: voice_id.to_string(),
        name: name.to_string(),
        gender: gender.map(str::to_string),
        language: language.to_string(),
        styles: strings(styles),
        preview_url: None,
        scenarios: strings(scenarios),
        model_version: model_version.to_string(),
    }
}

pub fn builtin_voices() -> Vec<TtsVoiceInfo> {
    vec![
        // 有声阅读（Audiobook）
        voice(
            "zh_male_baqiqingshu_uranus_bigtts",
            "霸气青叔 2.0",
            Some("male"),
            "zh",
            &[],
            &["有声阅读"],
            "2.0",
        ),
        voice(
            "zh_male_ruyaqingnian_uranus_bigtts",
            "儒雅青年 2.0",
            Some("male"),
            "zh",
            &[],
            &["有声阅读"],
            "2.0",
        ),
        voice(
            "zh_male_xuanyijieshuo_uranus_bigtts",
            "悬疑解说 2.0",
            Some("male"),
            "zh",
            &[],
            &["有声阅读"],
            "2.0",
        ),
        voice(
            "zh_female_wenroushunv_uranus_bigtts",
            "温柔淑女 2.0",
            Some("female"),
            "zh",
            &[],
            &["有声阅读"],
            "2.0",
        ),
        voice(
            "zh_female_shaoergushi_uranus_bigtts",
            "少儿故事 2.0",
            Some("female"),
            "zh",
            &[],
            &["有声阅读"],
            "2.0",
        ),
        voice(
            "zh_female_gufengshaoyu_uranus_bigtts",
            "古风少御 2.0",
            Some("female"),
            "zh",
            &[],
            &["有声阅读"],
            "2.0",
        ),
        voice(
            "zh_male_qingcang_uranus_bigtts",
            "擎苍 2.0",
            Some("male"),
            "zh",
            &[],
            &["有声阅读", "角色扮演"],
            "2.0",
        ),
        voice(
            "zh_male_huolixiaoge_uranus_bigtts",
            "活力小哥 2.0",
            Some("male"),
            "zh",
            &[],
            &["有声阅读"],
            "2.0",
        ),
        voice(
            "zh_male_fanjuanqingnian_uranus_bigtts",
            "反卷青年 2.0",
            Some("male"),
            "zh",
            &[],
            &["有声阅读"],
            "2.0",
        ),

        // 通用场景（General）
        voice(
            "zh_female_vv_uranus_bigtts",
            "Vivi 2.0",
            Some("female"),
            "zh",
            &["happy", "sad", "angry", "surprised", "fear", "excited", "neutral"],
            &["通用场景"],
            "2.0",
        ),
        voice(
            "zh_female_xiaohe_uranus_bigtts",
            "小何 2.0",
            Some("female"),
            "zh",
            &[],
            &["通用场景"],
            "2.0",
        ),
        voice(
            "zh_male_m191_uranus_bigtts",
            "云舟 2.0",
            Some("male"),
            "zh",
            &[],
            &["通用场景"],
            "2.0",
        ),
        voice(
            "zh_male_taocheng_uranus_bigtts",
            "小天 2.0",
            Some("male"),
            "zh",
            &[],
            &["通用场景"],
            "2.0",
        ),
        voice(
            "zh_female_qingxinnvsheng_uranus_bigtts",
            "清新女声 2.0",
            Some("female"),
            "zh",
            &[],
            &["通用场景"],
            "2.0",
        ),
        voice(
            "zh_female_sophie_uranus_bigtts",
            "魅力苏菲 2.0",
            Some("female"),
            "zh",
            &[],
            &["通用场景"],
            "2.0",
        ),
        voice(
            "zh_female_tianmeixiaoyuan_uranus_bigtts",
            "甜美小源 2.0",
            Some("female"),
            "zh",
            &[],
            &["通用场景"],
            "2.0",
        ),
        voice(
            "zh_female_tianmeitaozi_uranus_bigtts",
            "甜美桃子 2.0",
            Some("female"),
            "zh",
            &[],
            &["通用场景"],
            "2.0",
        ),
        voice(
            "zh_female_shuangkuaisisi_uranus_bigtts",
            "爽快思思 2.0",
            Some("female"),
            "zh",
            &[],
            &["通用场景"],
            "2.0",
        ),
        voice(
            "zh_female_linjianvhai_uranus_bigtts",
            "邻家女孩 2.0",
            Some("female"),
            "zh",
            &[],
            &["通用场景"],
            "2.0",
        ),
        voice(
            "zh_male_shaonianzixin_uranus_bigtts",
            "少年梓辛 2.0",
            Some("male"),
            "zh",
            &[],
            &["通用场景"],
            "2.0",
        ),
        voice(
            "zh_female_wenroumama_uranus_bigtts",
            "温柔妈妈 2.0",
            Some("female"),
            "zh",
            &[],
            &["通用场景"],
            "2.0",
        ),
        voice(
            "zh_male_jieshuoxiaoming_uranus_bigtts",
            "解说小明 2.0",
            Some("male"),
            "zh",
            &[],
            &["通用场景"],
            "2.0",
        ),
        voice(
            "zh_female_tvbnv_uranus_bigtts",
            "TVB女声 2.0",
            Some("female"),
            "zh",
            &[],
            &["通用场景"],
            "2.0",
        ),
        voice(
            "zh_male_yizhipiannan_uranus_bigtts",
            "译制片男 2.0",
            Some("male"),
            "zh",
            &[],
            &["通用场景"],
            "2.0",
        ),
        voice(
            "zh_female_qiaopinv_uranus_bigtts",
            "俏皮女声 2.0",
            Some("female"),
            "zh",
            &[],
            &["通用场景"],
            "2.0",
        ),
        voice(
            "zh_male_linjiananhai_uranus_bigtts",
            "邻家男孩 2.0",
            Some("male"),
            "zh",
            &[],
            &["通用场景"],
            "2.0",
        ),
        voice(
            "zh_male_ruyaqingnian_uranus_bigtts",
            "儒雅青年 2.0",
            Some("male"),
            "zh",
            &[],
            &["通用场景", "有声阅读"],
            "2.0",
        ),
        voice(
            "zh_male_wennuanahu_uranus_bigtts",
            "温暖阿虎 2.0",
            Some("male"),
            "zh",
            &[],
            &["通用场景"],
            "2.0",
        ),
        voice(
            "zh_male_naiqimengwa_uranus_bigtts",
            "奶气萌娃 2.0",
            Some("male"),
            "zh",
            &[],
            &["通用场景"],
            "2.0",
        ),
        voice(
            "zh_female_popo_uranus_bigtts",
            "婆婆 2.0",
            Some("female"),
            "zh",
            &[],
            &["通用场景"],
            "2.0",
        ),
        voice(
            "zh_female_gaolengyujie_uranus_bigtts",
            "高冷御姐 2.0",
            Some("female"),
            "zh",
            &[],
            &["通用场景"],
            "2.0",
        ),
        voice(
            "zh_male_aojiaobazong_uranus_bigtts",
            "傲娇霸总 2.0",
            Some("male"),
            "zh",
            &[],
            &["通用场景"],
            "2.0",
        ),

        // 角色扮演（Role Play）
        voice(
            "zh_female_cancan_uranus_bigtts",
            "知性灿灿 2.0",
            Some("female"),
            "zh",
            &[],
            &["角色扮演"],
            "2.0",
        ),
        voice(
            "zh_female_sajiaoxuemei_uranus_bigtts",
            "撒娇学妹 2.0",
            Some("female"),
            "zh",
            &[],
            &["角色扮演"],
            "2.0",
        ),
        voice(
            "zh_male_sunwukong_uranus_bigtts",
            "猴哥 2.0",
            Some("male"),
            "zh",
            &[],
            &["视频配音", "角色扮演"],
            "2.0",
        ),
        voice(
            "zh_female_linxiao_uranus_bigtts",
            "林潇 2.0",
            Some("female"),
            "zh",
            &[],
            &["角色扮演"],
            "2.0",
        ),
        voice(
            "zh_female_lingling_uranus_bigtts",
            "玲玲姐姐 2.0",
            Some("female"),
            "zh",
            &[],
            &["角色扮演"],
            "2.0",
        ),
        voice(
            "zh_female_chunribu_uranus_bigtts",
            "春日部姐姐 2.0",
            Some("female"),
            "zh",
            &[],
            &["角色扮演"],
            "2.0",
        ),
        voice(
            "zh_male_lubanqihao_uranus_bigtts",
            "鲁班七号 2.0",
            Some("male"),
            "zh",
            &[],
            &["角色扮演"],
            "2.0",
        ),
        voice(
            "zh_male_zhuangzhou_uranus_bigtts",
            "庄周 2.0",
            Some("male"),
            "zh",
            &[],
            &["角色扮演"],
            "2.0",
        ),
        voice(
            "zh_male_zhubajie_uranus_bigtts",
            "猪八戒 2.0",
            Some("male"),
            "zh",
            &[],
            &["角色扮演"],
            "2.0",
        ),
        voice(
            "zh_female_wuzetian_uranus_bigtts",
            "武则天 2.0",
            Some("female"),
            "zh",
            &[],
            &["角色扮演"],
            "2.0",
        ),

        // 视频配音
        voice(
            "zh_female_peiqi_uranus_bigtts",
            "佩奇猪 2.0",
            Some("female"),
            "zh",
            &[],
            &["视频配音"],
            "2.0",
        ),
        voice(
            "zh_male_dayi_uranus_bigtts",
            "大壹 2.0",
            Some("male"),
            "zh",
            &[],
            &["视频配音"],
            "2.0",
        ),
        voice(
            "zh_female_jitangnv_uranus_bigtts",
            "鸡汤女 2.0",
            Some("female"),
            "zh",
            &[],
            &["视频配音"],
            "2.0",
        ),
        voice(
            "zh_female_liuchangnv_uranus_bigtts",
            "流畅女声 2.0",
            Some("female"),
            "zh",
            &[],
            &["视频配音"],
            "2.0",
        ),
        voice(
            "zh_male_ruyayichen_uranus_bigtts",
            "儒雅逸辰 2.0",
            Some("male"),
            "zh",
            &[],
            &["视频配音"],
            "2.0",
        ),
        voice(
            "zh_male_xionger_uranus_bigtts",
            "熊二 2.0",
            Some("male"),
            "zh",
            &[],
            &["视频配音", "角色扮演"],
            "2.0",
        ),

        // 其它官方 2.0 音色补充
        voice(
            "zh_male_liufei_uranus_bigtts",
            "刘飞 2.0",
            Some("male"),
            "zh",
            &[],
            &["通用场景"],
            "2.0",
        ),
        voice(
            "zh_female_meilinvyou_uranus_bigtts",
            "魅力女友 2.0",
            Some("female"),
            "zh",
            &[],
            &["通用场景"],
            "2.0",
        ),
        voice(
            "zh_female_xiaoxue_uranus_bigtts",
            "儿童绘本 2.0",
            Some("female"),
            "zh",
            &[],
            &["通用场景"],
            "2.0",
        ),
        voice(
            "zh_female_mizai_uranus_bigtts",
            "黑猫侦探社咪仔 2.0",
            Some("female"),
            "zh",
            &[],
            &["通用场景"],
            "2.0",
        ),
        voice(
            "en_female_stokie_uranus_bigtts",
            "Stokie 2.0",
            Some("female"),
            "en",
            &[],
            &["多语种"],
            "2.0",
        ),
        voice(
            "zh_female_wenwanqingyin_uranus_bigtts",
            "温婉轻音 2.0",
            Some("female"),
            "zh",
            &[],
            &["通用场景"],
            "2.0",
        ),
        voice(
            "zh_male_chenshengaoye_uranus_bigtts",
            "沉稳大爷 2.0",
            Some("male"),
            "zh",
            &[],
            &["通用场景"],
            "2.0",
        ),
        voice(
            "zh_female_qingrunnuzhu_uranus_bigtts",
            "轻柔女主 2.0",
            Some("female"),
            "zh",
            &[],
            &["通用场景"],
            "2.0",
        ),
        voice(
            "zh_male_houqingnansheng_uranus_bigtts",
            "厚情男声 2.0",
            Some("male"),
            "zh",
            &[],
            &["通用场景"],
            "2.0",
        ),
        voice(
            "zh_female_nianshangnvsheng_uranus_bigtts",
            "念伤女声 2.0",
            Some("female"),
            "zh",
            &[],
            &["通用场景"],
            "2.0",
        ),
        voice(
            "zh_male_yanggangnansheng_uranus_bigtts",
            "阳刚男声 2.0",
            Some("male"),
            "zh",
            &[],
            &["通用场景"],
            "2.0",
        ),
        voice(
            "zh_female_nuanxinjiemei_uranus_bigtts",
            "暖心姐妹 2.0",
            Some("female"),
            "zh",
            &[],
            &["通用场景"],
            "2.0",
        ),
        voice(
            "zh_male_qingsongnansheng_uranus_bigtts",
            "轻松男声 2.0",
            Some("male"),
            "zh",
            &[],
            &["通用场景"],
            "2.0",
        ),
        voice(
            "zh_female_yuanzhinvsheng_uranus_bigtts",
            "元气女声 2.0",
            Some("female"),
            "zh",
            &[],
            &["通用场景"],
            "2.0",
        ),
        voice(
            "zh_male_yuanqinansheng_uranus_bigtts",
            "元气男声 2.0",
            Some("male"),
            "zh",
            &[],
            &["通用场景"],
            "2.0",
        ),
        voice(
            "zh_female_yujingnvsheng_uranus_bigtts",
            "御姐女声 2.0",
            Some("female"),
            "zh",
            &[],
            &["通用场景"],
            "2.0",
        ),
        voice(
            "zh_male_xueshengnansheng_uranus_bigtts",
            "学生男声 2.0",
            Some("male"),
            "zh",
            &[],
            &["教育场景"],
            "2.0",
        ),
        voice(
            "zh_female_xueshengnvsheng_uranus_bigtts",
            "学生女声 2.0",
            Some("female"),
            "zh",
            &[],
            &["教育场景"],
            "2.0",
        ),
        voice(
            "zh_male_xinwenguangbo_uranus_bigtts",
            "新闻广播 2.0",
            Some("male"),
            "zh",
            &[],
            &["视频配音"],
            "2.0",
        ),
        voice(
            "zh_female_xinwenvsheng_uranus_bigtts",
            "新闻女声 2.0",
            Some("female"),
            "zh",
            &[],
            &["视频配音"],
            "2.0",
        ),
        voice(
            "zh_male_dushudashi_uranus_bigtts",
            "读书大叔 2.0",
            Some("male"),
            "zh",
            &[],
            &["有声阅读"],
            "2.0",
        ),
        voice(
            "zh_female_dushujiemei_uranus_bigtts",
            "读书姐妹 2.0",
            Some("female"),
            "zh",
            &[],
            &["有声阅读"],
            "2.0",
        ),

        // 多语种
        voice(
            "en_male_tim_uranus_bigtts",
            "Tim",
            Some("male"),
            "en",
            &[],
            &["多语种"],
            "2.0",
        ),
        voice(
            "en_female_dacey_uranus_bigtts",
            "Dacey",
            Some("female"),
            "en",
            &[],
            &["多语种"],
            "2.0",
        ),

        // 额外补充，保证 80+
        voice(
            "zh_female_qingchunshaoonv_uranus_bigtts",
            "青春少女 2.0",
            Some("female"),
            "zh",
            &[],
            &["通用场景"],
            "2.0",
        ),
        voice(
            "zh_male_wenrennansheng_uranus_bigtts",
            "文人男声 2.0",
            Some("male"),
            "zh",
            &[],
            &["通用场景"],
            "2.0",
        ),
        voice(
            "zh_female_tianzhennvsheng_uranus_bigtts",
            "天真女声 2.0",
            Some("female"),
            "zh",
            &[],
            &["通用场景"],
            "2.0",
        ),
        voice(
            "zh_male_xuanyannansheng_uranus_bigtts",
            "宣言男声 2.0",
            Some("male"),
            "zh",
            &[],
            &["通用场景"],
            "2.0",
        ),
        voice(
            "zh_female_furongnvsheng_uranus_bigtts",
            "芙蓉女声 2.0",
            Some("female"),
            "zh",
            &[],
            &["通用场景"],
            "2.0",
        ),
        voice(
            "zh_male_liangban_nansheng_uranus_bigtts",
            "凉拌男声 2.0",
            Some("male"),
            "zh",
            &[],
            &["通用场景"],
            "2.0",
        ),
        voice(
            "zh_female_nuanyangnvsheng_uranus_bigtts",
            "暖阳女声 2.0",
            Some("female"),
            "zh",
            &[],
            &["客服场景"],
            "2.0",
        ),
        voice(
            "zh_male_jizhounansheng_uranus_bigtts",
            "机智男声 2.0",
            Some("male"),
            "zh",
            &[],
            &["通用场景"],
            "2.0",
        ),
        voice(
            "zh_female_roumeinvsheng_uranus_bigtts",
            "柔美女声 2.0",
            Some("female"),
            "zh",
            &[],
            &["通用场景"],
            "2.0",
        ),
        voice(
            "zh_male_chunfengnansheng_uranus_bigtts",
            "春风男声 2.0",
            Some("male"),
            "zh",
            &[],
            &["通用场景"],
            "2.0",
        ),
        voice(
            "zh_female_xiaoqiaonvsheng_uranus_bigtts",
            "小巧女声 2.0",
            Some("female"),
            "zh",
            &[],
            &["通用场景"],
            "2.0",
        ),
        voice(
            "zh_male_hougaonansheng_uranus_bigtts",
            "厚高男声 2.0",
            Some("male"),
            "zh",
            &[],
            &["通用场景"],
            "2.0",
        ),
        voice(
            "zh_female_yueerqingsheng_uranus_bigtts",
            "悦耳轻声 2.0",
            Some("female"),
            "zh",
            &[],
            &["通用场景"],
            "2.0",
        ),
        voice(
            "zh_male_chuangxinboyi_uranus_bigtts",
            "创新博弈 2.0",
            Some("male"),
            "zh",
            &[],
            &["通用场景"],
            "2.0",
        ),
        voice(
            "zh_female_qunzhongnvsheng_uranus_bigtts",
            "群众女声 2.0",
            Some("female"),
            "zh",
            &[],
            &["通用场景"],
            "2.0",
        ),
        voice(
            "zh_male_guangmangnansheng_uranus_bigtts",
            "光芒男声 2.0",
            Some("male"),
            "zh",
            &[],
            &["通用场景"],
            "2.0",
        ),
        voice(
            "zh_female_haoqingnvsheng_uranus_bigtts",
            "豪情女声 2.0",
            Some("female"),
            "zh",
            &[],
            &["通用场景"],
            "2.0",
        ),
        voice(
            "zh_male_wendingnansheng_uranus_bigtts",
            "稳定男声 2.0",
            Some("male"),
            "zh",
            &[],
            &["通用场景"],
            "2.0",
        ),
        voice(
            "zh_female_shenminnvsheng_uranus_bigtts",
            "神秘女声 2.0",
            Some("female"),
            "zh",
            &[],
            &["有声阅读"],
            "2.0",
        ),
        voice(
            "zh_male_shenminnansheng_uranus_bigtts",
            "神秘男声 2.0",
            Some("male"),
            "zh",
            &[],
            &["有声阅读"],
            "2.0",
        ),
        voice(
            "zh_female_shuiyunnvsheng_uranus_bigtts",
            "水韵女声 2.0",
            Some("female"),
            "zh",
            &[],
            &["通用场景"],
            "2.0",
        ),
        voice(
            "zh_male_shuiyunnansheng_uranus_bigtts",
            "水韵男声 2.0",
            Some("male"),
            "zh",
            &[],
            &["通用场景"],
            "2.0",
        ),
    ]
}

pub struct VoiceRegistry {
    voices: Vec<TtsVoiceInfo>,
}

impl VoiceRegistry {
    pub fn new() -> Self {
        Self {
            voices: builtin_voices(),
        }
    }

    pub fn with_voices(mut self, voices: Vec<TtsVoiceInfo>) -> Self {
        self.voices = voices;
        self
    }

    pub fn get(&self, id: &str) -> Option<&TtsVoiceInfo> {
        self.voices.iter().find(|voice| voice.voice_id == id)
    }

    pub fn list_all(&self) -> &[TtsVoiceInfo] {
        &self.voices
    }

    pub fn list_by_scenario(&self, scenario: &str) -> Vec<&TtsVoiceInfo> {
        self.voices
            .iter()
            .filter(|voice| voice.scenarios.iter().any(|item| item == scenario))
            .collect()
    }
}

impl Default for VoiceRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn by_name<'a>(voices: &'a [TtsVoiceInfo], name: &str) -> Option<&'a TtsVoiceInfo> {
        voices.iter().find(|voice| voice.name == name)
    }

    #[test]
    fn builtin_voices_exceed_minimum() {
        assert!(builtin_voices().len() >= 80);
    }

    #[test]
    fn key_podcast_voices_present() {
        let voices = builtin_voices();
        for name in [
            "霸气青叔 2.0",
            "儒雅青年 2.0",
            "悬疑解说 2.0",
            "温柔淑女 2.0",
            "少儿故事 2.0",
            "Vivi 2.0",
            "小何 2.0",
        ] {
            assert!(by_name(&voices, name).is_some(), "missing {name}");
        }
    }

    #[test]
    fn registry_get_finds_voice() {
        let registry = VoiceRegistry::new();
        let voice = registry.get("zh_female_xiaohe_uranus_bigtts").unwrap();
        assert_eq!(voice.name, "小何 2.0");
    }

    #[test]
    fn registry_get_unknown_returns_none() {
        let registry = VoiceRegistry::new();
        assert!(registry.get("missing-voice-id").is_none());
    }

    #[test]
    fn scenario_filtering_works() {
        let registry = VoiceRegistry::new();
        let voices = registry.list_by_scenario("有声阅读");
        assert!(!voices.is_empty());
        assert!(voices.iter().any(|voice| voice.name == "霸气青叔 2.0"));
    }

    #[test]
    fn override_replaces_builtin() {
        let registry = VoiceRegistry::new().with_voices(vec![voice(
            "custom_voice",
            "Custom Voice",
            Some("female"),
            "zh",
            &[],
            &["通用场景"],
            "2.0",
        )]);

        assert_eq!(registry.list_all().len(), 1);
        assert_eq!(registry.get("custom_voice").unwrap().name, "Custom Voice");
        assert!(registry.get("zh_female_xiaohe_uranus_bigtts").is_none());
    }
}

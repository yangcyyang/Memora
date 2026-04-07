# 错误排查记录 (Error Log)

## ❌ MiniMax TTS 音色克隆功能限制说明

### 故障现象与错误代码
在尝试通过 MiniMax 的 `/v1/voice_clone` API 进行音色克隆时，通常会遇到以下错误代码序列：

#### 1. Code 1004: login fail (API 密钥污染问题)
* **表现**: 发送请求时返回 `1004 login fail: Please carry the API secret key in the 'Authorization' field`。
* **原因**: 之前因为 `save_tts_settings` 参数错误产生的报错堆栈文本，被前端当成正常的 API Key 传给了后端，并由于缺乏格式校验，被后端作为合法密钥进行了 AES 加密并写入了 `tts_settings.json` 中。
* **解决办法**: 已在 `src-tauri/src/commands/tts.rs` 中增加了强格式校验代码（`is_ascii_alphanumeric` 以及横杠），防止中文或异常字符写入配置。如果用户遭遇此错误，只需去设置界面**重新保存一次正确的 API 密钥**即可。

#### 2. “Missing file_id in upload response” / TypeError 
* **表现**: 第二步上传音频获取 `file_id` 成功，但在解析返回的 JSON 时抛出 `Missing file_id` 错误。
* **原因**: MiniMax 的 `file_id` 返回的不是 `String` 而是纯数字（`Number`），用 `.as_str()` 提取会导致取值为 `None`。
* **解决办法**: 已在 `minimax.rs` 解析代码中增加了 `serde_json::Value` 类型自适应匹配，兼容将返回的 `Value::Number(n)` 也提取为后续交互需要的字符串格式。

#### 3. Code 2013: invalid params
* **表现**: 格式化错误修复后，发起最终业务克隆请求时报错 `invalid params`。
* **原因**: 我们由于内部逻辑将 `file_id` 当做 String 流转，在向 MiniMax API 的 Payload 发送 Json 时，也将其构造为了 `"file_id": "38497..."`。但 MiniMax API 是强类型的，必须要求这个字段为 `integer`。
* **解决办法**: 已通过 `.parse::<i64>().unwrap_or_else()` 强转回 `Number` 解决参数格式校验被强行拦截的问题。

#### 4. Code 2038: voice clone user forbidden (核心业务阻断原因)
* **表现**: 前面所有代码、秘钥、参数类型的校验全通后，业务接口强行终止了克隆进度。
* **终极原因**: **MinMax 官方主动限制！** 代表当前用户的 API 开发者账号，没有权限调用 `/v1/voice_clone`。在中国大陆基于合规和 AIGC 防欺诈的要求，MiniMax 以及部分国内厂商已经关闭了“个人开发者”的音色克隆接口权限。
* **结论**: **如果不前往 MiniMax 开放平台完成企业实名认证并提交人工白名单申请，任何普通账号都因为遇到 `2038` 错误而“无法使用”该接口。**

---
**当前建议**: 此并非 Memora 代码实现问题。如需长期使用克隆，建议配置经过认证的企业级 MiniMax 账号，或者开发替换接入策略更宽松的开源/其他商业 TTS 供应商（例如 Fish Audio, ElevenLabs）。

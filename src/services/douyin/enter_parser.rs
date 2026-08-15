//! enter 接口 JSON 解析

use crate::config::constants;
use crate::error::AppError;

/// enter 接口的完整解析结果
#[derive(Debug, Clone, Default)]
pub struct EnterRoomData {
    pub is_live: bool,
    pub room_title: Option<String>,
    pub sec_uid: Option<String>,
    pub nickname: Option<String>,
    pub avatar: Option<String>,
}

/// 解析 `webcast/room/web/enter/` 接口的 JSON 响应
///
/// 开播判定：
/// - `data.data` 数组非空：已开通直播间
/// - `data.room_status == 0`：正在直播
pub fn parse(body: &str) -> Result<EnterRoomData, AppError> {
    let v: serde_json::Value = serde_json::from_str(body)
        .map_err(|e| AppError::internal(format!("解析 JSON 失败: {e}")))?;

    let status_code = v.get("status_code").and_then(|x| x.as_i64()).unwrap_or(-1);
    if status_code != constants::ENTER_STATUS_OK {
        return Err(AppError::internal(format!(
            "抖音直播间接口返回 status_code: {status_code}"
        )));
    }

    let data = v
        .get("data")
        .ok_or_else(|| AppError::internal("响应缺少 data 字段"))?;

    let room_datas = data.get("data").and_then(|x| x.as_array());
    let has_room = room_datas.map(|a| !a.is_empty()).unwrap_or(false);

    let room_status = data
        .get("room_status")
        .and_then(|x| x.as_i64())
        .unwrap_or(-1);
    let is_live = has_room && room_status == constants::ROOM_STATUS_LIVING;

    let room_title = room_datas
        .and_then(|a| a.first())
        .and_then(|r| r.get("title"))
        .and_then(|x| x.as_str())
        .map(|s| s.to_string());

    let user = data.get("user");
    let sec_uid = user
        .and_then(|u| u.get("sec_uid"))
        .and_then(|x| x.as_str())
        .map(|s| s.to_string());
    let nickname = user
        .and_then(|u| u.get("nickname"))
        .and_then(|x| x.as_str())
        .map(|s| s.to_string());
    let avatar = user
        .and_then(|u| u.get("avatar_thumb"))
        .and_then(|a| a.get("url_list"))
        .and_then(|l| l.as_array())
        .and_then(|a| a.first())
        .and_then(|x| x.as_str())
        .map(|s| s.to_string());

    if !has_room {
        tracing::debug!("douyin enter: room data empty for sec_uid={:?}", sec_uid);
    }

    Ok(EnterRoomData {
        is_live,
        room_title,
        sec_uid,
        nickname,
        avatar,
    })
}

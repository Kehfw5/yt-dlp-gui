import i18n from "@/locales";
import { NText, NFlex, NCollapse, NCollapseItem } from "naive-ui";

const t = i18n.global.t;

/** 将错误消息转换为用户友好的提示 */
export const formatError = (raw: string): string => {
  // 解析 Rust 返回的 error key（格式：err_key 或 err_key:detail）
  const errMatch = raw.match(/^(err_[a-z_]+)(?::(.*))?$/);
  if (errMatch) {
    const [, key, detail] = errMatch;
    const i18nKey = `errors.${key}`;
    const translated = t(i18nKey);
    // 如果 i18n 有对应翻译（不等于 key 本身），使用翻译
    if (translated !== i18nKey) {
      return detail ? `${translated}: ${detail}` : translated;
    }
  }

  // yt-dlp 原生错误匹配
  if (/Unsupported URL/i.test(raw)) return t("errors.unsupportedUrl");
  if (/is not a valid URL/i.test(raw)) return t("errors.invalidUrl");
  if (/Video unavailable/i.test(raw)) return t("errors.videoUnavailable");
  if (/Private video/i.test(raw)) return t("errors.privateVideo");
  if (/HTTP Error 403/i.test(raw)) return t("errors.http403");
  if (/HTTP Error 404/i.test(raw)) return t("errors.http404");
  if (/HTTP Error 429/i.test(raw)) return t("errors.http429");
  if (/No video formats found/i.test(raw)) return t("errors.noFormats");
  if (/Could not copy.*cookie database/i.test(raw)) return t("errors.cookieDatabaseLocked");
  if (/Unable to download webpage/i.test(raw)) return t("errors.cannotDownload");
  if (/urlopen error|Connection refused|timed out/i.test(raw)) return t("errors.networkError");
  // 清理 ERROR: [extractor] id: 前缀
  return raw.replace(/^ERROR:\s*(\[.*?\]\s*\w+:\s*)?/i, "").trim() || raw;
};

/** 用 dialog 弹出错误信息，支持展开原始错误 */
export const showErrorDialog = (raw: string) => {
  const friendly = formatError(raw);
  // 清理后的原始信息（去掉 ERROR: 前缀），用于判断是否有额外信息值得展示
  const cleaned = raw
    .trim()
    .replace(/^ERROR:\s*(\[.*?\]\s*\w+:\s*)?/i, "")
    .trim();
  // 只有翻译后的友好信息和清理后的原始信息不同时，才展示可展开的原始详情
  const hasRawDetail = friendly !== cleaned && raw.trim().length > 0;

  if (!hasRawDetail) {
    window.$dialog.error({
      title: t("errors.dialogTitle"),
      content: friendly,
      positiveText: t("errors.dialogOk"),
    });
    return;
  }

  window.$dialog.error({
    title: t("errors.dialogTitle"),
    content: () =>
      h(NFlex, { vertical: true, size: 8 }, () => [
        h(NText, null, () => friendly),
        h(NCollapse, { arrowPlacement: "left" }, () =>
          h(NCollapseItem, { title: t("errors.showDetail"), name: "detail" }, () =>
            h(
              NText,
              {
                depth: 3,
                style:
                  "font-size: 12px; word-break: break-all; white-space: pre-wrap; max-height: 200px; overflow-y: auto; display: block; user-select: text; cursor: text;",
              },
              () => raw.trim(),
            ),
          ),
        ),
      ]),
    positiveText: t("errors.dialogOk"),
  });
};

/** 格式化文件大小为可读字符串（B / KB / MB / GB） */
export const formatFileSize = (bytes: number): string => {
  const units = ["B", "KB", "MB", "GB"];
  let size = bytes;
  let i = 0;
  while (size >= 1024 && i < units.length - 1) {
    size /= 1024;
    i++;
  }
  return `${size.toFixed(1)} ${units[i]}`;
};

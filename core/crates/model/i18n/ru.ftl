# Tactus — русские строки UI / озвучки (Fluent).
# Идентификаторы через '-'; точки в i18n-ключах профиля нормализуются в '-'.

kit-label = Кит { $number }: { $name }

param-tempo = { $value } уд/мин
param-kit-name = { $name }
param-kit-sub-name = { $name }
param-kit-num = Кит { $number }
param-tempo-switch = Переключатель темпа: { $value }

# Лейблы параметров (подписи контролов / для скринридера — без значения).
param-tempo-label = Темп
param-kit-name-label = Имя кита
param-kit-sub-name-label = Доп. имя
param-kit-num-label = Кит
param-tempo-switch-label = Переключатель темпа

instrument-name = { $name }
instrument-unknown = Инструмент №{ $number } (неизвестен)

edit-mismatch = Не удалось изменить — осталось { $value }.
edit-timeout = Нет ответа — значение неизвестно. Проверьте подключение.
edit-out-of-range = Значение вне диапазона.
edit-not-ready = Нет подключения к устройству.

device-connected = Подключено: { $device }, прошивка { $firmware }.
device-firmware-untested = Эта прошивка не в списке протестированных Tactus — должно работать; сообщите о проблемах.
device-unrecognized = Подключён нераспознанный модуль. Часть функций может быть недоступна.

# ── Интерфейс самого приложения (ADR-0008: единый источник формулировок) ──
ui-section-connection = Подключение
ui-label-status = Состояние
ui-label-device = Устройство
ui-label-firmware = Прошивка
ui-status-disconnected = Нет подключения
ui-status-identifying = Определение…
ui-status-ready = Готово
ui-connect-prompt = Подключите барабанный модуль кабелем USB.
ui-firmware-newer = Эта прошивка новее протестированных. Всё должно работать.
ui-firmware-older = Эта прошивка старее протестированных. Всё должно работать.
ui-firmware-unknown = Эта прошивка не тестировалась. Всё должно работать.

ui-section-kit = Кит
ui-label-current-kit = Текущий кит
ui-value-current-kit = Текущий кит: { $value }
ui-button-previous-kit = Предыдущий кит
ui-button-next-kit = Следующий кит
ui-button-rename-kit = Переименовать кит…
ui-hint-rename-kit = Изменить имя текущего кита
ui-title-rename-kit = Переименование кита
ui-label-kit-name = Имя кита
ui-button-save = Сохранить
ui-button-cancel = Отмена

ui-section-tempo = Темп
ui-label-tempo = Темп
ui-value-updating = Обновление…
ui-hint-tempo-adjust = Проведите вверх или вниз, чтобы изменить темп
ui-value-unknown = —

ui-section-language = Язык
ui-language-system = Системный

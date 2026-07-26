# Tactus — русские строки UI / озвучки (Fluent).
# Идентификаторы через '-'; точки в i18n-ключах профиля нормализуются в '-'.

kit-label = Кит { $number }: { $name }
# Ответ на «следующий/предыдущий кит» на краю списка китов модуля.
kit-at-first = Первый кит.
kit-at-last = Последний кит.

param-tempo = { $value } уд/мин
param-kit-name = { $name }
param-kit-sub-name = { $name }
param-kit-num = Кит { $value }
param-tempo-switch = Переключатель темпа: { $value }
param-setlist-name = { $name }
param-setlist-step = Кит { $value }
# Сырое значение с собственным смыслом (см. `sentinel` в профиле).
value-setlist-end = Конец сет-листа

# Лейблы параметров (подписи контролов / для скринридера — без значения).
param-tempo-label = Темп
param-kit-name-label = Имя кита
param-kit-sub-name-label = Доп. имя
param-kit-num-label = Кит
param-setlist-name-label = Имя сет-листа
param-setlist-step-label = Шаг
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

# ── Параметры кита ──
# Значение-перечисление — это собственное слово модуля (OFF, WARM HALL,
# SRV-2000): произносится дословно и помечается английским, как и написано на
# экране модуля и в руководстве Roland. Переводим только подписи ниже.
param-enum-value = { $value }

param-kit-volume = { $value } дБ
param-kit-volume-label = Громкость кита

param-unit-volume = { $value } дБ
param-unit-volume-label = Громкость пэда
param-unit-overhead-send = { $value } дБ
param-unit-overhead-send-label = Посыл на оверхеды
param-unit-room-send = { $value } дБ
param-unit-room-send-label = Посыл на комнату
param-unit-reverb-send = { $value } дБ
param-unit-reverb-send-label = Посыл на реверберацию

param-layer-switch-label = Слой
param-layer-instrument = { $value }
param-layer-instrument-label = Инструмент
param-layer-inst-bank = { $value }
param-layer-inst-bank-label = Банк инструментов
param-layer-volume = { $value } дБ
param-layer-volume-label = Громкость слоя
param-layer-pitch = { $value } центов
param-layer-pitch-label = Высота тона
param-layer-decay = { $value }
param-layer-decay-label = Затухание

param-pad-pan = { $value }
param-pad-pan-label = Панорама

param-fx-type = { $value }
param-fx-type-label = Тип эффекта
param-fx-switch-label = Эффект

param-overhead-switch-label = Оверхеды
param-overhead-mic-type-label = Тип микрофонов оверхед
param-overhead-level = { $value } дБ
param-overhead-level-label = Уровень оверхедов

param-room-switch-label = Комната
param-room-type-label = Тип комнаты
param-room-level = { $value } дБ
param-room-level-label = Уровень комнаты

param-reverb-switch-label = Реверберация
param-reverb-type-label = Тип реверберации
param-reverb-level = { $value } дБ
param-reverb-level-label = Уровень реверберации

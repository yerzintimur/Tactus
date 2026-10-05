# Tactus — русские строки UI / озвучки (Fluent).
# Идентификаторы через '-'; точки в i18n-ключах профиля нормализуются в '-'.

kit-label = Кит { $number }: { $name }
# Ответ на «следующий/предыдущий кит» на краю списка китов модуля.
kit-at-first = Первый кит.
kit-at-last = Последний кит.
setlist-step-kit = Шаг { $step }, кит { $number }: { $name }
setlist-at-first = Первый шаг сет-листа.
setlist-at-last = Последний шаг сет-листа.
setlist-empty = Сет-лист пуст.

param-tempo = { $value } уд/мин
param-kit-name = { $name }
param-kit-sub-name = { $name }
param-kit-num = Кит { $value }
param-tempo-switch = Переключатель темпа: { $value }
param-setlist-name = { $name }
param-setlist-step = Кит { $value }
# Сырые значения с собственным смыслом (см. `sentinel` в профиле).
value-setlist-end = Конец сет-листа
# Уровень на самом дне: модуль показывает -INF, и ничего не слышно.
value-level-silent = Тишина

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
# Банк, для которого у нас нет каталога — ещё не описанный пак расширения.
instrument-unknown-bank = Инструмент №{ $number } в банке { $bank } (неизвестен)

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
# Настройка модуля: переключатели, которые может включить только барабанщик
# на самом модуле; стоят внизу главного экрана, пока не увидим каждый
# включённым. Transmit Edit Data нет в адресной карте; значение — путь по
# меню модуля, как его даёт профиль.
ui-section-setup = Настройка модуля
ui-hint-transmit-edit-data = Включите на модуле Transmit Edit Data ({ $value }), чтобы слышать, что вы меняете на его панели.

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

ui-section-setlist = Сет-листы
ui-value-setlist-number = Сет-лист { $value }
ui-label-setlist-name = Имя сет-листа
ui-value-setlist-step = Шаг { $value }
ui-value-setlist-empty = В этом сет-листе пока нет китов
ui-hint-setlist = Читать и переставлять киты сет-листа
ui-button-add-current-kit = Добавить текущий кит
ui-button-move-step-up = Вверх
ui-button-move-step-down = Вниз
ui-button-remove-step = Убрать
ui-button-rename-setlist = Переименовать сет-лист…
ui-title-rename-setlist = Переименование сет-листа
ui-button-previous-step = Предыдущий шаг
ui-button-next-step = Следующий шаг
ui-value-setlist-current-step = { $value }, текущий шаг

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

# Имена позиций, по которым повторяется параметр (`dimensions` профиля).
# Произносятся перед параметром: «Обод малого, Слой A, Громкость слоя: 0.5 дБ».
# Ключи следуют Data List Roland, где зоны любого пэда называются HEAD/RIM
# (у райда ещё EDGE и BELL); озвучка — словами барабанщика.
pad-kick = Бочка
pad-snare = Малый
pad-snare-head = Пластик малого
pad-snare-rim = Обод малого
pad-tom1 = Том 1
pad-tom1-head = Пластик тома 1
pad-tom1-rim = Обод тома 1
pad-tom2 = Том 2
pad-tom2-head = Пластик тома 2
pad-tom2-rim = Обод тома 2
pad-tom3 = Том 3
pad-tom3-head = Пластик тома 3
pad-tom3-rim = Обод тома 3
pad-tom4 = Том 4
pad-tom4-head = Пластик тома 4
pad-tom4-rim = Обод тома 4
pad-hihat = Хай-хэт
pad-hihat-head = Боу хай-хэта
pad-hihat-rim = Край хай-хэта
pad-crash1 = Крэш 1
pad-crash1-head = Боу крэша 1
pad-crash1-rim = Край крэша 1
pad-crash2 = Крэш 2
pad-crash2-head = Боу крэша 2
pad-crash2-rim = Край крэша 2
pad-ride = Райд
pad-ride-head = Боу райда
pad-ride-edge = Край райда
pad-ride-bell = Колокол райда
# Первый дополнительный вход модуль подписывает AUX, остальные — AUX2–AUX4.
pad-aux = Aux
pad-aux-head = Пластик Aux
pad-aux-rim = Обод Aux
pad-aux2 = Aux 2
pad-aux2-head = Пластик Aux 2
pad-aux2-rim = Обод Aux 2
pad-aux3 = Aux 3
pad-aux3-head = Пластик Aux 3
pad-aux3-rim = Обод Aux 3
pad-aux4 = Aux 4
pad-aux4-head = Пластик Aux 4
pad-aux4-rim = Обод Aux 4

layer-a = Слой A
layer-b = Слой B
layer-c = Слой C

# Слоты эффектов кита: по два на шину, четыре шины (BUS-A FX1 … BUS-D FX2).
fx-bus-a-1 = Шина A, эффект 1
fx-bus-a-2 = Шина A, эффект 2
fx-bus-b-1 = Шина B, эффект 1
fx-bus-b-2 = Шина B, эффект 2
fx-bus-c-1 = Шина C, эффект 1
fx-bus-c-2 = Шина C, эффект 2
fx-bus-d-1 = Шина D, эффект 1
fx-bus-d-2 = Шина D, эффект 2

# Позиция, у которой есть только номер (с 1, как на экране модуля).
dim-step = Шаг { $number }

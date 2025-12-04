@echo off
chcp 65001 >nul
echo === Тестирование API ===
echo.

echo 1. Получение списка моделей:
curl http://127.0.0.1:3000/models
echo.
echo.

echo 2. Получение списка пресетов:
curl http://127.0.0.1:3000/presets
echo.
echo.

echo 3. Тест price_classifier - iPhone 15:
echo Запрос: {"prompt": "iPhone 15", "preset": "price_classifier"}
curl -X POST http://127.0.0.1:3000/chat -H "Content-Type: application/json" -d "{\"prompt\": \"iPhone 15\", \"preset\": \"price_classifier\"}"
echo.
echo.

echo 4. Тест price_classifier - вонючие трусы:
echo Запрос: {"prompt": "вонючие трусы", "preset": "price_classifier"}
curl -X POST http://127.0.0.1:3000/chat -H "Content-Type: application/json" -d "{\"prompt\": \"вонючие трусы\", \"preset\": \"price_classifier\"}"
echo.
echo.

echo 5. Тест price_classifier - хлеб:
echo Запрос: {"prompt": "хлеб", "preset": "price_classifier"}
curl -X POST http://127.0.0.1:3000/chat -H "Content-Type: application/json" -d "{\"prompt\": \"хлеб\", \"preset\": \"price_classifier\"}"
echo.
echo.

echo 6. Тест sentiment - позитивный отзыв:
echo Запрос: {"prompt": "Отличный товар, очень доволен!", "preset": "sentiment"}
curl -X POST http://127.0.0.1:3000/chat -H "Content-Type: application/json" -d "{\"prompt\": \"Отличный товар, очень доволен!\", \"preset\": \"sentiment\"}"
echo.
echo.

echo 7. Тест assistant:
echo Запрос: {"prompt": "Что такое Rust?", "preset": "assistant"}
curl -X POST http://127.0.0.1:3000/chat -H "Content-Type: application/json" -d "{\"prompt\": \"Что такое Rust?\", \"preset\": \"assistant\"}"
echo.
echo.

echo 8. Тест date_extractor - явная дата:
echo Запрос: {"prompt": "Встреча назначена на 15 марта 2024 года", "preset": "date_extractor"}
curl -X POST http://127.0.0.1:3000/chat -H "Content-Type: application/json" -d "{\"prompt\": \"Встреча назначена на 15 марта 2024 года\", \"preset\": \"date_extractor\"}"
echo.
echo.

echo 9. Тест date_extractor - дата в формате:
echo Запрос: {"prompt": "Концерт состоится 31.12.2024 в 19:00", "preset": "date_extractor"}
curl -X POST http://127.0.0.1:3000/chat -H "Content-Type: application/json" -d "{\"prompt\": \"Концерт состоится 31.12.2024 в 19:00\", \"preset\": \"date_extractor\"}"
echo.
echo.

echo 10. Тест date_extractor - относительная дата:
echo Запрос: {"prompt": "Доставка будет завтра", "preset": "date_extractor"}
curl -X POST http://127.0.0.1:3000/chat -H "Content-Type: application/json" -d "{\"prompt\": \"Доставка будет завтра\", \"preset\": \"date_extractor\"}"
echo.
echo.

pause

import { BrowserRouter, Routes, Route } from "react-router-dom";
import { ConfigProvider } from "antd";
import { valetTheme } from "./theme";
import { AppLayout } from "./components/AppLayout";
import { ProfileProvider } from "./contexts/ProfileProvider";

function App() {
  return (
    <BrowserRouter>
      <ConfigProvider theme={valetTheme}>
        <ProfileProvider>
          <Routes>
            <Route path="*" element={<AppLayout />} />
          </Routes>
        </ProfileProvider>
      </ConfigProvider>
    </BrowserRouter>
  );
}

export default App;
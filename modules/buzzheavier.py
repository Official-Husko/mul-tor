# Import Standard Libraries
import os
import random

# Import Third-Party Libraries
import requests

# Import Local Libraries


class Buzzheavier:
    def __init__(self, file: str, user_agent: str, proxy_list: list[str]=None, api_key: list=None) -> None:
        self.file = file
        self.proxy = random.choice(proxy_list) if proxy_list else None
        self.user_agent = user_agent
        self.file_name = os.path.basename(file)
        self.api_key = api_key

        # Below is the website specific data
        self.site: str = "Buzzheavier"
        self.api_key_required: bool = False
        self.site_url: str = "https://buzzheavier.com/"
        self.upload_url: str = "https://w.buzzheavier.com/"
        self.download_url_base: str = "https://buzzheavier.com/f/"

        self.headers: dict = {
            "User-Agent": user_agent,
            "Accept": "*/*",
            "Content-Length": str(os.path.getsize(file))
        }

    def uploader(self) -> dict:
        try:
            with open(self.file, "rb") as file_upload:
                full_url = f"{self.upload_url}{self.file_name}?expiry=10368000"

                file_contents = file_upload.read()

                response = requests.put(
                    full_url,
                    data=file_contents,
                    headers=self.headers,
                    proxies=self.proxy,
                    timeout=300
                )

                if not response.status == 201:
                    raise Exception(response.status)
                    
                try:
                    response_json = response.json()
                    file_id = response_json.get("id")
                    return {"status": "ok", "file_name": self.file_name, "file_url": f"{self.download_url_base}{file_id}"}
                except Exception as e:
                    return {"status": "error", "file_name": self.file_name, "exception": str(e), "extra": f"Failed to get json data. Code: {response.status} | {response.data}"}    

        except Exception as e:
            # Return error message
            return {"status": "error", "file_name": self.file_name, "exception": str(e), "extra": f"Upload Failed. Code fucked up."}
    
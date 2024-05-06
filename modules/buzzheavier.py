# Import Standard Packages
import os
import random

# Import Third Party Packages
import requests

# Import Custom Modules
from main import DEBUG


class Buzzheavier:
    def __init__(self, file: str, proxy_list: list, user_agent: str, api_key: list):
        self.file = file
        self.proxy = random.choice(proxy_list) if proxy_list else None
        self.user_agent = user_agent
        self.file_name = os.path.basename(file)
        self.api_key = api_key

        # Below is the website specific data
        self.site = "Buzzheavier"
        self.api_key_required = False
        self.site_url = "https://buzzheavier.com/"
        self.upload_url = "https://w.buzzheavier.com/"
        self.download_url_base = "https://buzzheavier.com/f/"

        self.headers = {
            "User-Agent": user_agent,
            "Accept": "*/*",
            "Content-Length": str(os.path.getsize(file))
        }

    def uploader(self):
        try:
            with open(self.file, "rb") as file_upload:
                full_url = f"{self.upload_url}{self.file_name}?expiry=10368000"

                file_contents = file_upload.read()

                print(type(file_contents))

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
    
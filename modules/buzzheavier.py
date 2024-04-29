import urllib3
import os
import random

from .pretty_print import *
from main import DEBUG


class Buzzheavier:
    def __init__(self, file, proxy_list, user_agent, api_key):
        self.file = file
        self.proxy = random.choice(proxy_list) if proxy_list else None
        self.user_agent = user_agent
        self.file_name = os.path.basename(file)

        # Below is the website specific data
        self.site = "Buzzheavier"
        self.apiKey_req = False
        self.site_url = "https://buzzheavier.com/"
        self.upload_url = "https://w.buzzheavier.com/"
        self.download_url_base = "https://buzzheavier.com/f/"

        self.headers = {
            "User-Agent": user_agent,
            "Accept": "*/*",
            "Content-Length": str(os.path.getsize(file))
        }

        self.http = urllib3.PoolManager()

        self.Uploader()


    def Uploader(self):
        try:
            with open(self.file, "rb") as file_upload:
                full_url = f"{self.upload_url}{self.file_name}?expiry=10368000"

                file_contents = file_upload.read()

                print(type(file_contents))

                response = self.http.request(
                    "PUT",
                    full_url,
                    body=file_contents,
                    headers=self.headers,
                    retries=3,
                    timeout=300,
                    redirect=False
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
    
import requests
import os
import random

from .site_data import Site_Data_CLSS, sites_data_dict
from .pretty_print import *
from main import DEBUG

site = "Buzzheavier"

class Buzzheavier:
     def Uploader(file, proxy_list, user_agents, api_keys):
        """
        Uploads a file to a specified site using random user agents and proxies.
        Args:
            file (str): The path to the file to be uploaded.
            proxy_list (list): A list of proxy URLs.
            user_agents (list): A list of user agent strings.
        Returns:
            dict: A dictionary containing the status, file name, file URL, and site.
        Raises:
            Exception: If an error occurs during the upload process.
        """
        raw_req = "None :("
        try:
            # Select a random user agent
            ua = random.choice(user_agents)
            upload_url = sites_data_dict[site]["url"]

            # Get the file size and name
            file_name = os.path.basename(file)

            # Set the user agent header
            headers = {
                "User-Agent": "curl/8.4.0",
                "Accept": "*/*",
                "Content-Length": str(os.path.getsize(file))
            }

            # Truncate the file name if it is too long
            file_name = (file_name[:240] + '..') if len(file_name) > 240 else file_name

            # Select a random proxy, if available
            proxies = random.choice(proxy_list) if proxy_list else None

            # Send the upload request with the form data, headers, and proxies
            with open(file, "rb") as file_upload:
                raw_req = requests.put(url=f"{upload_url}{file_name}", data=file_upload, headers=headers, proxies=proxies, timeout=300, stream=True)
                file_upload.close()
            if raw_req.status_code == 200:

                try:
                    raw_req = raw_req.json()
                    download_url = raw_req.get("url")

                except Exception as e:
                    return {"status": "error", "file_name": file_name, "exception": str(e), "extra": raw_req.text}

                return {"status": "ok", "file_name": file_name, "file_url": download_url}
            else:
                raise Exception(raw_req.status_code)

            file_id = raw_req.text
            download_url = download_url_base + file_id.replace("&", "_")

            # Return successful message with the status, file name, file URL, and site
            return {"status": "ok", "file_name": file_name, "file_url": download_url}

        except Exception as e:
            # Return error message
            return {"status": "error", "file_name": file_name, "exception": str(e), "extra": raw_req}
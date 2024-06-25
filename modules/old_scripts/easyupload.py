import requests
import os
import random
import uuid
import re
import json

from .site_data import Site_Data_CLSS, sites_data_dict
from .pretty_print import *
from main import DEBUG

site = "EasyUpload"
raw_req = "None :("

class EasyUpload:
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

            size_limit = f'{sites_data_dict[site]["size_limit"]} {sites_data_dict[site]["size_unit"]}'

            # Get the file size and name
            file_size = os.stat(file).st_size
            file_name = os.path.basename(file)

            # Get the upload URL and size limit from the site data dictionary
            initialize_url = sites_data_dict[site]["initialize_url"]

            # Truncate the file name if it is too long
            file_name = (file_name[:240] + '..') if len(file_name) > 240 else file_name

            # Calculate the size unit for the site
            calc_size = Site_Data_CLSS.size_unit_calc(site, file_size)

            # Select a random proxy, if available
            proxies = random.choice(proxy_list) if proxy_list else None

            # This will get the actual upload URL from the website
            normal_url = sites_data_dict[site]["api_url"]

            raw_req = requests.get(url=normal_url, headers={"User-Agent": ua}, proxies=proxies, timeout=300)

            pattern = r"https://upload\d+\.easyupload\.io/action\.php"
            match = re.search(pattern, raw_req.text)

            if match:
                upload_url = match.group(0)
            else:
                raise Exception("Server URL Missing. Report this!")

            # Set the user agent header
            headers = {
                "User-Agent": ua,
                "Accept": "application/json",
                "Referer": "https://easyupload.io/",
            }

            file_uuid = str(uuid.uuid4())

            pattern = r'\{.*\}'

            if calc_size == "OK":
                # Prepare the form data for file upload
                init_data = {
                    "type": "validate_tuf",
                    "files[]": file_uuid
                }
                # Send the upload request with the form data, headers, and proxies
                init_req = requests.post(url=initialize_url, data=init_data, headers=headers, proxies=proxies, timeout=300)

                # Parse the response JSON and get the download URL
                match = re.search(pattern, init_req.text)
                json_part = match.group()
                init_resp = json.loads(json_part)
                status = init_resp.get("status", "")

                if status != True:
                    raise Exception(f"Init status: {status}")

                chunk_size = 100000000  # 100 MB
                total_chunks = (file_size + chunk_size - 1) // chunk_size
                chunk_index = 0
                dzchunkbyteoffset = 0

                with open(file, 'rb') as file_data:
                    while True:
                        chunk_data = file_data.read(chunk_size)
                        if not chunk_data:
                            break  # Exit the loop if we've reached the end of the file
                        
                        upload_data = {
                            "dzuuid": file_uuid,
                            "dzchunkindex": chunk_index,
                            "dztotalfilesize": file_size,
                            "dzchunksize": chunk_size,
                            "dztotalchunkcount": total_chunks,
                            "dzchunkbyteoffset": dzchunkbyteoffset,
                            "type": "dropzone-multi-upload",
                            "expiration": "30"
                        }

                        # Prepare the json data to add extra data to the upload
                        form_data = {
                                    'file': (os.path.basename(file), chunk_data, 'application/octet-stream')
                                }

                        # Send the upload request with the form data, headers, and proxies
                        raw_req = requests.post(url=upload_url, data=upload_data, files=form_data, headers=headers, proxies=proxies, timeout=300, stream=True)

                        chunk_index += 1
                        dzchunkbyteoffset += chunk_size

                match = re.search(pattern, raw_req.text)
                json_part = match.group()
                upload_resp = json.loads(json_part)

                download_url = upload_resp.get("download_link", "")

                # Return successful message with the status, file name, file URL, and site
                return {"status": "ok", "file_name": file_name, "file_url": download_url}
            else:
                # Return size error message
                return {"status": "size_error", "file_name": file_name, "exception": "SIZE_ERROR", "size_limit": f"{str(size_limit)}"}
        except Exception as e:
            # Return error message
            return {"status": "error", "file_name": file_name, "exception": str(e), "extra": raw_req}


import requests
import os
import random
import base64

from .site_data import Site_Data_CLSS, sites_data_dict
from .pretty_print import *
from main import DEBUG

site = "FileTransfer"

class FileTransfer:
     def Uploader(file, proxy_list, user_agents, api_key):
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

            # Set the user agent header
            headers = {
                "User-Agent": ua,
                "Accept": "application/json",
                "X-Requested-With": "XMLHttpRequest"
            }

            # Select a random proxy, if available
            proxies = random.choice(proxy_list) if proxy_list else None

            if calc_size == "OK":
                # Send the upload request with the form data, headers, and proxies
                init_req = requests.get(url=initialize_url, headers=headers, proxies=proxies)

                # Parse the response JSON and get the download URL
                init_resp = init_req.json()
                upload_url = init_resp.get("uploadUrl", "")

                chunk_size = 10485760  # 10 MB (WHO THE FUCK DOES THIS SHIT IN 10MB CHUNKS? THIS WEBSITE IS FUCKED)
                chunk_offset = 0

                with open(file, 'rb') as file_data:
                    while True:
                        chunk_data = file_data.read(chunk_size)
                        if not chunk_data:
                            break  # Exit the loop if we've reached the end of the file
                        
                        base64_filename = base64.b64encode(file_name.encode('utf-8'))

                        headers = {
                            "User-Agent": ua,
                            "Tus-Resumable": "1.0.0",
                            "Upload-Length": f"{file_size}",
                            "Upload-Metadata": f"filename {base64_filename},filetype YXBwbGljYXRpb24vb2N0ZXQtc3RyZWFt,fileorder MQ==",
                            "Upload-Offset": f"{chunk_offset}"
                        }

                        # Prepare the json data to add extra data to the upload
                        form_data = {
                                    'file': (os.path.basename(file), chunk_data, 'application/octet-stream')
                                }

                        # Send the upload request with the form data, headers, and proxies
                        raw_req = requests.post(url=upload_url, files=form_data, headers=headers, proxies=proxies, timeout=50)

                        if raw_req.status_code == 200:
                            print("chunk uploaded")

                        chunk_offset += chunk_size

                headers = {
                    "User-Agent": ua,
                    "X-Requested-With": "XMLHttpRequest"
                }

                raw_req = requests.post(url=upload_url, headers=headers, proxies=proxies, timeout=50)
                print(raw_req.text)

                upload_resp = raw_req.json()

                download_url = upload_resp.get("deliveryPublicLink", "")

                # Return successful message with the status, file name, file URL, and site
                return {"status": "ok", "file_name": file_name, "file_url": download_url}
            else:
                # Return size error message
                return {"status": "size_error", "file_name": file_name, "exception": "SIZE_ERROR", "size_limit": f"{str(size_limit)}"}
        except Exception as e:
            # Return error message
            return {"status": "error", "file_name": file_name, "exception": str(e), "extra": raw_req.content}

"""

Author: Husko
Date: 06/10/2023

"""
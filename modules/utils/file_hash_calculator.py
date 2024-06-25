import hashlib
import os

class FileHashCalculator:
    """
    A class to calculate hash values of files.

    Attributes:
        file (str): The path to the file.
        hash_type (hashlib._Hash): The type of hash algorithm to be used.
        hash (str): The hash value of the file.
    """

    def __init__(self, file: str):
        """
        Initializes the FileHashCalculator object.

        Args:
            file (str): The path to the file.
        """
        self.file = file
        self.hash_type = hashlib.md5(usedforsecurity=False)
        self.hash = self._get_file_hash()

    def _get_file_hash(self):
        """
        Calculates the hash value of the file.

        Returns:
            str: The hash value of the file.
        """

        with open(os.path.basename(self.file), "rb") as file_hc:
            # Read the file in chunks to handle large files
            for chunk in iter(lambda: file_hc.read(4096), b""):
                self.hash_type.update(chunk)
        
        return self.hash_type.hexdigest()

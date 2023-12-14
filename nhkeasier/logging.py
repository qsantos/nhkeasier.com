import logging
from logging.handlers import TimedRotatingFileHandler
from time import sleep

from django.core.mail import send_mail


class DjangoMailHandler(logging.Handler):
    def emit(self, record: logging.LogRecord) -> None:
        msg = self.format(record)
        for _ in range(10):  # try sending the error several times
            try:
                send_mail(
                    subject=f'[{record.levelname}] {record.msg}',
                    message=f'{record.pathname}:{record.lineno}\n{msg}',
                    from_email='logs@nhkeasier.com',
                    recipient_list=['contact@nhkeasier.com'],
                )
            except OSError:
                logging.debug('FAILED TO SEND MAIL ALERT', exc_info=True)
                sleep(5)
                continue
            else:
                break


def log_to_console(logger: logging.Logger, level: int, formatter: logging.Formatter) -> None:
    handler = logging.StreamHandler()
    handler.setLevel(level)
    handler.setFormatter(formatter)
    logger.addHandler(handler)


def log_to_file(logger: logging.Logger, level: int, formatter: logging.Formatter) -> None:
    handler = TimedRotatingFileHandler('nhkeasier.com.log', when='D')
    handler.setLevel(level)
    handler.setFormatter(formatter)
    logger.addHandler(handler)


def log_to_mail(logger: logging.Logger, level: int, formatter: logging.Formatter) -> None:
    handler = DjangoMailHandler()
    handler.setLevel(level)
    handler.setFormatter(formatter)
    logger.addHandler(handler)


def init_logging() -> None:
    logger = logging.getLogger()
    logger.setLevel(logging.DEBUG)

    formatter = logging.Formatter(
        '%(asctime)s - %(name)s - %(levelname)s - %(message)s',
    )

    log_to_console(logger, logging.INFO, formatter)
    log_to_file(logger, logging.DEBUG, formatter)
    log_to_mail(logger, logging.WARNING, formatter)

    logger = logging.getLogger(__name__)
    logger.debug('Logging initialized')

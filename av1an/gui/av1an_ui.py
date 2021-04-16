# -*- coding: utf-8 -*-

# Form implementation generated from reading ui file 'av1an_ui.ui'
#
# Created by: PyQt5 UI code generator 5.15.1
#
# WARNING: Any manual changes made to this file will be lost when pyuic5 is
# run again.  Do not edit this file unless you know what you are doing.

from PyQt5 import QtCore, QtGui, QtWidgets


class Ui_Av1an_main_windows(object):
    def setupUi(self, Av1an_main_windows):
        Av1an_main_windows.setObjectName("Av1an_main_windows")
        Av1an_main_windows.resize(960, 600)
        sizePolicy = QtWidgets.QSizePolicy(
            QtWidgets.QSizePolicy.MinimumExpanding,
            QtWidgets.QSizePolicy.MinimumExpanding,
        )
        sizePolicy.setHorizontalStretch(200)
        sizePolicy.setVerticalStretch(200)
        sizePolicy.setHeightForWidth(
            Av1an_main_windows.sizePolicy().hasHeightForWidth()
        )
        Av1an_main_windows.setSizePolicy(sizePolicy)
        Av1an_main_windows.setMinimumSize(QtCore.QSize(960, 600))
        self.centralwidget = QtWidgets.QWidget(Av1an_main_windows)
        sizePolicy = QtWidgets.QSizePolicy(
            QtWidgets.QSizePolicy.Expanding, QtWidgets.QSizePolicy.Expanding
        )
        sizePolicy.setHorizontalStretch(200)
        sizePolicy.setVerticalStretch(200)
        sizePolicy.setHeightForWidth(
            self.centralwidget.sizePolicy().hasHeightForWidth()
        )
        self.centralwidget.setSizePolicy(sizePolicy)
        self.centralwidget.setObjectName("centralwidget")
        self.horizontalLayout_3 = QtWidgets.QHBoxLayout(self.centralwidget)
        self.horizontalLayout_3.setObjectName("horizontalLayout_3")
        self.verticalLayout = QtWidgets.QVBoxLayout()
        self.verticalLayout.setObjectName("verticalLayout")
        self.ButtonSelect = QtWidgets.QHBoxLayout()
        self.ButtonSelect.setContentsMargins(10, 10, 10, 10)
        self.ButtonSelect.setObjectName("ButtonSelect")
        self.Input_Button = QtWidgets.QPushButton(self.centralwidget)
        self.Input_Button.setObjectName("Input_Button")
        self.ButtonSelect.addWidget(self.Input_Button)
        spacerItem = QtWidgets.QSpacerItem(
            10, 20, QtWidgets.QSizePolicy.Expanding, QtWidgets.QSizePolicy.Minimum
        )
        self.ButtonSelect.addItem(spacerItem)
        self.Video_Settings_Button = QtWidgets.QPushButton(self.centralwidget)
        self.Video_Settings_Button.setObjectName("Video_Settings_Button")
        self.ButtonSelect.addWidget(self.Video_Settings_Button)
        spacerItem1 = QtWidgets.QSpacerItem(
            10, 20, QtWidgets.QSizePolicy.Expanding, QtWidgets.QSizePolicy.Minimum
        )
        self.ButtonSelect.addItem(spacerItem1)
        self.Encoder_Button = QtWidgets.QPushButton(self.centralwidget)
        self.Encoder_Button.setObjectName("Encoder_Button")
        self.ButtonSelect.addWidget(self.Encoder_Button)
        spacerItem2 = QtWidgets.QSpacerItem(
            10, 20, QtWidgets.QSizePolicy.Expanding, QtWidgets.QSizePolicy.Minimum
        )
        self.ButtonSelect.addItem(spacerItem2)
        self.OutputButton = QtWidgets.QPushButton(self.centralwidget)
        self.OutputButton.setObjectName("OutputButton")
        self.ButtonSelect.addWidget(self.OutputButton)
        self.ButtonSelect.setStretch(0, 1)
        self.ButtonSelect.setStretch(2, 1)
        self.ButtonSelect.setStretch(4, 1)
        self.ButtonSelect.setStretch(6, 1)
        self.verticalLayout.addLayout(self.ButtonSelect)
        self.MainWidget = QtWidgets.QStackedWidget(self.centralwidget)
        self.MainWidget.setMinimumSize(QtCore.QSize(800, 400))
        self.MainWidget.setObjectName("MainWidget")
        self.page = QtWidgets.QWidget()
        sizePolicy = QtWidgets.QSizePolicy(
            QtWidgets.QSizePolicy.Expanding, QtWidgets.QSizePolicy.Expanding
        )
        sizePolicy.setHorizontalStretch(100)
        sizePolicy.setVerticalStretch(100)
        sizePolicy.setHeightForWidth(self.page.sizePolicy().hasHeightForWidth())
        self.page.setSizePolicy(sizePolicy)
        self.page.setObjectName("page")
        self.horizontalLayout_4 = QtWidgets.QHBoxLayout(self.page)
        self.horizontalLayout_4.setObjectName("horizontalLayout_4")
        self.DropInLabel = QtWidgets.QLabel(self.page)
        sizePolicy = QtWidgets.QSizePolicy(
            QtWidgets.QSizePolicy.Expanding, QtWidgets.QSizePolicy.Expanding
        )
        sizePolicy.setHorizontalStretch(100)
        sizePolicy.setVerticalStretch(100)
        sizePolicy.setHeightForWidth(self.DropInLabel.sizePolicy().hasHeightForWidth())
        self.DropInLabel.setSizePolicy(sizePolicy)
        font = QtGui.QFont()
        font.setPointSize(36)
        self.DropInLabel.setFont(font)
        self.DropInLabel.setAutoFillBackground(False)
        self.DropInLabel.setStyleSheet(
            "            QLabel{\n"
            "\n"
            "                border: 4px dashed #aaa\n"
            "\n"
            "            }"
        )
        self.DropInLabel.setFrameShape(QtWidgets.QFrame.Box)
        self.DropInLabel.setScaledContents(True)
        self.DropInLabel.setAlignment(QtCore.Qt.AlignCenter)
        self.DropInLabel.setWordWrap(True)
        self.DropInLabel.setObjectName("DropInLabel")
        self.horizontalLayout_4.addWidget(self.DropInLabel)
        self.MainWidget.addWidget(self.page)
        self.page_2 = QtWidgets.QWidget()
        sizePolicy = QtWidgets.QSizePolicy(
            QtWidgets.QSizePolicy.Expanding, QtWidgets.QSizePolicy.Expanding
        )
        sizePolicy.setHorizontalStretch(100)
        sizePolicy.setVerticalStretch(100)
        sizePolicy.setHeightForWidth(self.page_2.sizePolicy().hasHeightForWidth())
        self.page_2.setSizePolicy(sizePolicy)
        self.page_2.setObjectName("page_2")
        self.horizontalLayout_5 = QtWidgets.QHBoxLayout(self.page_2)
        self.horizontalLayout_5.setObjectName("horizontalLayout_5")
        self.gen_set_wid = QtWidgets.QWidget(self.page_2)
        self.gen_set_wid.setObjectName("gen_set_wid")
        self.horizontalLayout_7 = QtWidgets.QHBoxLayout(self.gen_set_wid)
        self.horizontalLayout_7.setObjectName("horizontalLayout_7")
        self.pushButton_4 = QtWidgets.QPushButton(self.gen_set_wid)
        sizePolicy = QtWidgets.QSizePolicy(
            QtWidgets.QSizePolicy.MinimumExpanding,
            QtWidgets.QSizePolicy.MinimumExpanding,
        )
        sizePolicy.setHorizontalStretch(0)
        sizePolicy.setVerticalStretch(0)
        sizePolicy.setHeightForWidth(self.pushButton_4.sizePolicy().hasHeightForWidth())
        self.pushButton_4.setSizePolicy(sizePolicy)
        self.pushButton_4.setMinimumSize(QtCore.QSize(200, 200))
        font = QtGui.QFont()
        font.setPointSize(30)
        self.pushButton_4.setFont(font)
        self.pushButton_4.setObjectName("pushButton_4")
        self.horizontalLayout_7.addWidget(self.pushButton_4)
        self.horizontalLayout_5.addWidget(self.gen_set_wid)
        self.MainWidget.addWidget(self.page_2)
        self.page_3 = QtWidgets.QWidget()
        self.page_3.setObjectName("page_3")
        self.horizontalLayout = QtWidgets.QHBoxLayout(self.page_3)
        self.horizontalLayout.setObjectName("horizontalLayout")
        self.widget_2 = QtWidgets.QWidget(self.page_3)
        self.widget_2.setObjectName("widget_2")
        self.horizontalLayout_6 = QtWidgets.QHBoxLayout(self.widget_2)
        self.horizontalLayout_6.setObjectName("horizontalLayout_6")
        self.pushButton_3 = QtWidgets.QPushButton(self.widget_2)
        sizePolicy = QtWidgets.QSizePolicy(
            QtWidgets.QSizePolicy.MinimumExpanding,
            QtWidgets.QSizePolicy.MinimumExpanding,
        )
        sizePolicy.setHorizontalStretch(0)
        sizePolicy.setVerticalStretch(0)
        sizePolicy.setHeightForWidth(self.pushButton_3.sizePolicy().hasHeightForWidth())
        self.pushButton_3.setSizePolicy(sizePolicy)
        self.pushButton_3.setMinimumSize(QtCore.QSize(200, 200))
        font = QtGui.QFont()
        font.setPointSize(30)
        self.pushButton_3.setFont(font)
        self.pushButton_3.setObjectName("pushButton_3")
        self.horizontalLayout_6.addWidget(self.pushButton_3)
        self.horizontalLayout.addWidget(self.widget_2)
        self.MainWidget.addWidget(self.page_3)
        self.page_4 = QtWidgets.QWidget()
        self.page_4.setObjectName("page_4")
        self.horizontalLayout_2 = QtWidgets.QHBoxLayout(self.page_4)
        self.horizontalLayout_2.setObjectName("horizontalLayout_2")
        self.pushButton = QtWidgets.QPushButton(self.page_4)
        sizePolicy = QtWidgets.QSizePolicy(
            QtWidgets.QSizePolicy.MinimumExpanding,
            QtWidgets.QSizePolicy.MinimumExpanding,
        )
        sizePolicy.setHorizontalStretch(0)
        sizePolicy.setVerticalStretch(0)
        sizePolicy.setHeightForWidth(self.pushButton.sizePolicy().hasHeightForWidth())
        self.pushButton.setSizePolicy(sizePolicy)
        self.pushButton.setMinimumSize(QtCore.QSize(200, 200))
        font = QtGui.QFont()
        font.setPointSize(30)
        self.pushButton.setFont(font)
        self.pushButton.setObjectName("pushButton")
        self.horizontalLayout_2.addWidget(self.pushButton)
        self.MainWidget.addWidget(self.page_4)
        self.verticalLayout.addWidget(self.MainWidget)
        self.Progress_Bar = QtWidgets.QHBoxLayout()
        self.Progress_Bar.setContentsMargins(10, 10, 10, 10)
        self.Progress_Bar.setObjectName("Progress_Bar")
        self.progressBar = QtWidgets.QProgressBar(self.centralwidget)
        sizePolicy = QtWidgets.QSizePolicy(
            QtWidgets.QSizePolicy.Expanding, QtWidgets.QSizePolicy.Fixed
        )
        sizePolicy.setHorizontalStretch(100)
        sizePolicy.setVerticalStretch(0)
        sizePolicy.setHeightForWidth(self.progressBar.sizePolicy().hasHeightForWidth())
        self.progressBar.setSizePolicy(sizePolicy)
        self.progressBar.setProperty("value", 24)
        self.progressBar.setObjectName("progressBar")
        self.Progress_Bar.addWidget(self.progressBar)
        spacerItem3 = QtWidgets.QSpacerItem(
            20, 20, QtWidgets.QSizePolicy.Expanding, QtWidgets.QSizePolicy.Minimum
        )
        self.Progress_Bar.addItem(spacerItem3)
        self.StartStop = QtWidgets.QPushButton(self.centralwidget)
        self.StartStop.setObjectName("StartStop")
        self.Progress_Bar.addWidget(self.StartStop)
        self.Progress_Bar.setStretch(0, 1)
        self.verticalLayout.addLayout(self.Progress_Bar)
        self.horizontalLayout_3.addLayout(self.verticalLayout)
        Av1an_main_windows.setCentralWidget(self.centralwidget)

        self.retranslateUi(Av1an_main_windows)
        self.MainWidget.setCurrentIndex(0)
        QtCore.QMetaObject.connectSlotsByName(Av1an_main_windows)

    def retranslateUi(self, Av1an_main_windows):
        _translate = QtCore.QCoreApplication.translate
        Av1an_main_windows.setWindowTitle(
            _translate("Av1an_main_windows", "MainWindow")
        )
        self.Input_Button.setText(_translate("Av1an_main_windows", "Input"))
        self.Video_Settings_Button.setText(
            _translate("Av1an_main_windows", "Generic Settings")
        )
        self.Encoder_Button.setText(_translate("Av1an_main_windows", "Encoder"))
        self.OutputButton.setText(_translate("Av1an_main_windows", "Output"))
        self.DropInLabel.setText(
            _translate("Av1an_main_windows", "Drop In File Or Click For Import")
        )
        self.pushButton_4.setText(_translate("Av1an_main_windows", "GENERAL SETTINGS"))
        self.pushButton_3.setText(_translate("Av1an_main_windows", "Encoder Stuff"))
        self.pushButton.setText(_translate("Av1an_main_windows", "OUTPUT STUFF"))
        self.StartStop.setText(_translate("Av1an_main_windows", "Start"))

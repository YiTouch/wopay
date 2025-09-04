// SPDX-License-Identifier: MIT
pragma solidity ^0.8.21;

contract Wopay {

    // 已支付的订单
    mapping ( uint256 => bool ) public orderPaid;
    // 支付事件
    event PaymentReceived(uint256 indexed orderId, uint256 amount, address indexed sender);

    // 可以接收转账
    // 转账时需要传入订单号
    // 收到转账时需要触发日志并记录订单号
    // 订单号能否重复
    // 需要有提款功能：1. 提款权限如何确定 2. 如果是多商户，如何设计提款

    function receivePayment(uint256 orderId) external payable { 
        // 订单必须未支付
        require(!orderPaid[orderId], "Order already paid.");
        // 付款金额必须大于0
        require(msg.value > 0, "Payment must be greater than 0.");
        // 标记订单为已支付
        orderPaid[orderId] = true;

        emit PaymentReceived(orderId, msg.value, msg.sender);
    }
    
    
    function balance() external view returns(uint256){
        return address(this).balance;
    }

}
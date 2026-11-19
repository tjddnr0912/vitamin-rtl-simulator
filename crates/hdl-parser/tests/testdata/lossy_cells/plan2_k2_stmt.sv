module t; logic [69:0] b1,b2,b3,b4,b5,b6,b8; logic [34:0] b7h,b7l; wire [69:0] n1, n2; logic c; int i;
  assign #1 n1 = 'z; assign n2 = c ? '1 : 'z;
  initial begin c=1; i=0; b4=0; b5=0; b6=0; b8=0;
    b1 <= #1 'z; force b2 = 'z; assign b3 = '1; b4 |= 'z; b5 ^= '1; b6[39:5] = 'z; {b7h,b7l} = '1; b8[i +: 35] = '1;
    #5 $display("A b1=%h b2=%h b3=%h b4=%h", b1,b2,b3,b4); $display("B b5=%h b6=%h b7=%h%h b8=%h", b5,b6,b7h,b7l,b8); $display("C n1=%h n2=%h", n1,n2); end
  initial #100 $finish;
endmodule

program pg; logic [69:0] pv = 'x; logic [69:0] pw;
  initial begin pw = 'z; #1 $display("A pv=%h pw=%h", pv, pw); end
endprogram
module t; pg p(); initial #100 $finish; endmodule

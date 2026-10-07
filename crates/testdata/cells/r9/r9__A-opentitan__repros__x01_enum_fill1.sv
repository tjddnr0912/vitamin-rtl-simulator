module t;
  typedef enum logic [2:0] { OK = 3'h0, ERR = 3'h1, ALL = '1 } sts_e;
  sts_e s;
  initial begin s = ALL; #1 $display("A s=%b", s); $finish; end
endmodule

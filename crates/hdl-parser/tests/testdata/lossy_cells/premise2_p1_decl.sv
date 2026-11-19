module t;
  logic [69:0] a = 'x; logic [69:0] b = 'z; logic [69:0] ac = 70'bx;
  wire [69:0] n1 = 'z;
  time tm = 'x; logic signed [69:0] s = 'z; logic [3:0][69:0] pk = 'x;
  typedef logic [99:0] w_t; w_t tw = 'z; logic [39:0] m40 = 'x; logic [64:0] m65 = 'z; integer ig = 'z;
  logic [69:0] cst = 'x;
  initial begin #1 $display("A a=%h b=%h ac=%h", a, b, ac); $display("B n1=%h", n1);
    $display("C tm=%h s=%h tw=%h m40=%h m65=%h ig=%h cst=%h", tm, s, tw, m40, m65, ig, cst); $display("D pk=%h", pk); end
  initial #100 $finish;
endmodule

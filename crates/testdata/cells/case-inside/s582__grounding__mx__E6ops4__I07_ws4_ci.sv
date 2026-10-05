module top;
  logic signed [3:0] a, b; int m0, m1, m2, m3;
  initial begin
    a = 4'sh7; b = 4'sh7;
    case (a + b) inside 4'sb1?10: m0 = 1; default: m0 = 0; endcase
    case (a + b) inside 4'sb1?10: m1 = 1; 8'h55: m1 = 2; default: m1 = 0; endcase
    case (a + b) inside 4'sb1?10: m2 = 1; 16'sh7777: m2 = 2; default: m2 = 0; endcase
    case (a + b) inside 4'sb1?10: m3 = 1; 32'd12345: m3 = 2; default: m3 = 0; endcase
    $display("7+7 %0d %0d %0d %0d", m0, m1, m2, m3);
    a = 4'shF; b = 4'shF;
    case (a + b) inside 4'sb1?10: m0 = 1; default: m0 = 0; endcase
    case (a + b) inside 4'sb1?10: m1 = 1; 8'h55: m1 = 2; default: m1 = 0; endcase
    case (a + b) inside 4'sb1?10: m2 = 1; 16'sh7777: m2 = 2; default: m2 = 0; endcase
    case (a + b) inside 4'sb1?10: m3 = 1; 32'd12345: m3 = 2; default: m3 = 0; endcase
    $display("F+F %0d %0d %0d %0d", m0, m1, m2, m3);
    a = 4'sh8; b = 4'sh8;
    case (a + b) inside 4'sb1?10: m0 = 1; default: m0 = 0; endcase
    case (a + b) inside 4'sb1?10: m1 = 1; 8'h55: m1 = 2; default: m1 = 0; endcase
    case (a + b) inside 4'sb1?10: m2 = 1; 16'sh7777: m2 = 2; default: m2 = 0; endcase
    case (a + b) inside 4'sb1?10: m3 = 1; 32'd12345: m3 = 2; default: m3 = 0; endcase
    $display("8+8 %0d %0d %0d %0d", m0, m1, m2, m3);
    a = 4'sh0; b = 4'sh0;
    case (a + b) inside 4'sb1?10: m0 = 1; default: m0 = 0; endcase
    case (a + b) inside 4'sb1?10: m1 = 1; 8'h55: m1 = 2; default: m1 = 0; endcase
    case (a + b) inside 4'sb1?10: m2 = 1; 16'sh7777: m2 = 2; default: m2 = 0; endcase
    case (a + b) inside 4'sb1?10: m3 = 1; 32'd12345: m3 = 2; default: m3 = 0; endcase
    $display("0+0 %0d %0d %0d %0d", m0, m1, m2, m3);
    a = 4'sh7; b = 4'sh1;
    case (a + b) inside 4'sb1?10: m0 = 1; default: m0 = 0; endcase
    case (a + b) inside 4'sb1?10: m1 = 1; 8'h55: m1 = 2; default: m1 = 0; endcase
    case (a + b) inside 4'sb1?10: m2 = 1; 16'sh7777: m2 = 2; default: m2 = 0; endcase
    case (a + b) inside 4'sb1?10: m3 = 1; 32'd12345: m3 = 2; default: m3 = 0; endcase
    $display("7+1 %0d %0d %0d %0d", m0, m1, m2, m3);
    a = 4'shE; b = 4'sh0;
    case (a + b) inside 4'sb1?10: m0 = 1; default: m0 = 0; endcase
    case (a + b) inside 4'sb1?10: m1 = 1; 8'h55: m1 = 2; default: m1 = 0; endcase
    case (a + b) inside 4'sb1?10: m2 = 1; 16'sh7777: m2 = 2; default: m2 = 0; endcase
    case (a + b) inside 4'sb1?10: m3 = 1; 32'd12345: m3 = 2; default: m3 = 0; endcase
    $display("E+0 %0d %0d %0d %0d", m0, m1, m2, m3);
    a = 4'shF; b = 4'shE;
    case (a + b) inside 4'sb1?10: m0 = 1; default: m0 = 0; endcase
    case (a + b) inside 4'sb1?10: m1 = 1; 8'h55: m1 = 2; default: m1 = 0; endcase
    case (a + b) inside 4'sb1?10: m2 = 1; 16'sh7777: m2 = 2; default: m2 = 0; endcase
    case (a + b) inside 4'sb1?10: m3 = 1; 32'd12345: m3 = 2; default: m3 = 0; endcase
    $display("F+E %0d %0d %0d %0d", m0, m1, m2, m3);
    a = 4'sh1; b = 4'sh1;
    case (a + b) inside 4'sb1?10: m0 = 1; default: m0 = 0; endcase
    case (a + b) inside 4'sb1?10: m1 = 1; 8'h55: m1 = 2; default: m1 = 0; endcase
    case (a + b) inside 4'sb1?10: m2 = 1; 16'sh7777: m2 = 2; default: m2 = 0; endcase
    case (a + b) inside 4'sb1?10: m3 = 1; 32'd12345: m3 = 2; default: m3 = 0; endcase
    $display("1+1 %0d %0d %0d %0d", m0, m1, m2, m3);
    #10 $finish;
  end
endmodule

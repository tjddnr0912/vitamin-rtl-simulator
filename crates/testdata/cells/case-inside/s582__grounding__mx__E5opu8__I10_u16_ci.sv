module top;
  logic [7:0] a, b; int m0, m1, m2, m3;
  initial begin
    a = 8'h80; b = 8'h80;
    case (a + b) inside 16'h0100: m0 = 1; default: m0 = 0; endcase
    case (a + b) inside 16'h0100: m1 = 1; 8'h55: m1 = 2; default: m1 = 0; endcase
    case (a + b) inside 16'h0100: m2 = 1; 16'sh7777: m2 = 2; default: m2 = 0; endcase
    case (a + b) inside 16'h0100: m3 = 1; 32'd12345: m3 = 2; default: m3 = 0; endcase
    $display("80+80 %0d %0d %0d %0d", m0, m1, m2, m3);
    a = 8'hFF; b = 8'hFF;
    case (a + b) inside 16'h0100: m0 = 1; default: m0 = 0; endcase
    case (a + b) inside 16'h0100: m1 = 1; 8'h55: m1 = 2; default: m1 = 0; endcase
    case (a + b) inside 16'h0100: m2 = 1; 16'sh7777: m2 = 2; default: m2 = 0; endcase
    case (a + b) inside 16'h0100: m3 = 1; 32'd12345: m3 = 2; default: m3 = 0; endcase
    $display("FF+FF %0d %0d %0d %0d", m0, m1, m2, m3);
    a = 8'h7F; b = 8'h7F;
    case (a + b) inside 16'h0100: m0 = 1; default: m0 = 0; endcase
    case (a + b) inside 16'h0100: m1 = 1; 8'h55: m1 = 2; default: m1 = 0; endcase
    case (a + b) inside 16'h0100: m2 = 1; 16'sh7777: m2 = 2; default: m2 = 0; endcase
    case (a + b) inside 16'h0100: m3 = 1; 32'd12345: m3 = 2; default: m3 = 0; endcase
    $display("7F+7F %0d %0d %0d %0d", m0, m1, m2, m3);
    a = 8'h00; b = 8'h00;
    case (a + b) inside 16'h0100: m0 = 1; default: m0 = 0; endcase
    case (a + b) inside 16'h0100: m1 = 1; 8'h55: m1 = 2; default: m1 = 0; endcase
    case (a + b) inside 16'h0100: m2 = 1; 16'sh7777: m2 = 2; default: m2 = 0; endcase
    case (a + b) inside 16'h0100: m3 = 1; 32'd12345: m3 = 2; default: m3 = 0; endcase
    $display("00+00 %0d %0d %0d %0d", m0, m1, m2, m3);
    a = 8'hFF; b = 8'h01;
    case (a + b) inside 16'h0100: m0 = 1; default: m0 = 0; endcase
    case (a + b) inside 16'h0100: m1 = 1; 8'h55: m1 = 2; default: m1 = 0; endcase
    case (a + b) inside 16'h0100: m2 = 1; 16'sh7777: m2 = 2; default: m2 = 0; endcase
    case (a + b) inside 16'h0100: m3 = 1; 32'd12345: m3 = 2; default: m3 = 0; endcase
    $display("FF+01 %0d %0d %0d %0d", m0, m1, m2, m3);
    a = 8'hFE; b = 8'h00;
    case (a + b) inside 16'h0100: m0 = 1; default: m0 = 0; endcase
    case (a + b) inside 16'h0100: m1 = 1; 8'h55: m1 = 2; default: m1 = 0; endcase
    case (a + b) inside 16'h0100: m2 = 1; 16'sh7777: m2 = 2; default: m2 = 0; endcase
    case (a + b) inside 16'h0100: m3 = 1; 32'd12345: m3 = 2; default: m3 = 0; endcase
    $display("FE+00 %0d %0d %0d %0d", m0, m1, m2, m3);
    a = 8'h80; b = 8'h7F;
    case (a + b) inside 16'h0100: m0 = 1; default: m0 = 0; endcase
    case (a + b) inside 16'h0100: m1 = 1; 8'h55: m1 = 2; default: m1 = 0; endcase
    case (a + b) inside 16'h0100: m2 = 1; 16'sh7777: m2 = 2; default: m2 = 0; endcase
    case (a + b) inside 16'h0100: m3 = 1; 32'd12345: m3 = 2; default: m3 = 0; endcase
    $display("80+7F %0d %0d %0d %0d", m0, m1, m2, m3);
    a = 8'h00; b = 8'hFE;
    case (a + b) inside 16'h0100: m0 = 1; default: m0 = 0; endcase
    case (a + b) inside 16'h0100: m1 = 1; 8'h55: m1 = 2; default: m1 = 0; endcase
    case (a + b) inside 16'h0100: m2 = 1; 16'sh7777: m2 = 2; default: m2 = 0; endcase
    case (a + b) inside 16'h0100: m3 = 1; 32'd12345: m3 = 2; default: m3 = 0; endcase
    $display("00+FE %0d %0d %0d %0d", m0, m1, m2, m3);
    #10 $finish;
  end
endmodule

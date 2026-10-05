module top;
  logic [7:0] e; int m0, m1, m2, m3;
  initial begin
    e = 8'h00;
    case (e) inside 8'hFC: m0 = 1; default: m0 = 0; endcase
    case (e) inside 8'hFC: m1 = 1; 8'h55: m1 = 2; default: m1 = 0; endcase
    case (e) inside 8'hFC: m2 = 1; 16'sh7777: m2 = 2; default: m2 = 0; endcase
    case (e) inside 8'hFC: m3 = 1; 32'd12345: m3 = 2; default: m3 = 0; endcase
    $display("00 %0d %0d %0d %0d", m0, m1, m2, m3);
    e = 8'h0E;
    case (e) inside 8'hFC: m0 = 1; default: m0 = 0; endcase
    case (e) inside 8'hFC: m1 = 1; 8'h55: m1 = 2; default: m1 = 0; endcase
    case (e) inside 8'hFC: m2 = 1; 16'sh7777: m2 = 2; default: m2 = 0; endcase
    case (e) inside 8'hFC: m3 = 1; 32'd12345: m3 = 2; default: m3 = 0; endcase
    $display("0E %0d %0d %0d %0d", m0, m1, m2, m3);
    e = 8'h0F;
    case (e) inside 8'hFC: m0 = 1; default: m0 = 0; endcase
    case (e) inside 8'hFC: m1 = 1; 8'h55: m1 = 2; default: m1 = 0; endcase
    case (e) inside 8'hFC: m2 = 1; 16'sh7777: m2 = 2; default: m2 = 0; endcase
    case (e) inside 8'hFC: m3 = 1; 32'd12345: m3 = 2; default: m3 = 0; endcase
    $display("0F %0d %0d %0d %0d", m0, m1, m2, m3);
    e = 8'hFE;
    case (e) inside 8'hFC: m0 = 1; default: m0 = 0; endcase
    case (e) inside 8'hFC: m1 = 1; 8'h55: m1 = 2; default: m1 = 0; endcase
    case (e) inside 8'hFC: m2 = 1; 16'sh7777: m2 = 2; default: m2 = 0; endcase
    case (e) inside 8'hFC: m3 = 1; 32'd12345: m3 = 2; default: m3 = 0; endcase
    $display("FE %0d %0d %0d %0d", m0, m1, m2, m3);
    e = 8'hFF;
    case (e) inside 8'hFC: m0 = 1; default: m0 = 0; endcase
    case (e) inside 8'hFC: m1 = 1; 8'h55: m1 = 2; default: m1 = 0; endcase
    case (e) inside 8'hFC: m2 = 1; 16'sh7777: m2 = 2; default: m2 = 0; endcase
    case (e) inside 8'hFC: m3 = 1; 32'd12345: m3 = 2; default: m3 = 0; endcase
    $display("FF %0d %0d %0d %0d", m0, m1, m2, m3);
    e = 8'h80;
    case (e) inside 8'hFC: m0 = 1; default: m0 = 0; endcase
    case (e) inside 8'hFC: m1 = 1; 8'h55: m1 = 2; default: m1 = 0; endcase
    case (e) inside 8'hFC: m2 = 1; 16'sh7777: m2 = 2; default: m2 = 0; endcase
    case (e) inside 8'hFC: m3 = 1; 32'd12345: m3 = 2; default: m3 = 0; endcase
    $display("80 %0d %0d %0d %0d", m0, m1, m2, m3);
    e = 8'h7F;
    case (e) inside 8'hFC: m0 = 1; default: m0 = 0; endcase
    case (e) inside 8'hFC: m1 = 1; 8'h55: m1 = 2; default: m1 = 0; endcase
    case (e) inside 8'hFC: m2 = 1; 16'sh7777: m2 = 2; default: m2 = 0; endcase
    case (e) inside 8'hFC: m3 = 1; 32'd12345: m3 = 2; default: m3 = 0; endcase
    $display("7F %0d %0d %0d %0d", m0, m1, m2, m3);
    e = 8'hFA;
    case (e) inside 8'hFC: m0 = 1; default: m0 = 0; endcase
    case (e) inside 8'hFC: m1 = 1; 8'h55: m1 = 2; default: m1 = 0; endcase
    case (e) inside 8'hFC: m2 = 1; 16'sh7777: m2 = 2; default: m2 = 0; endcase
    case (e) inside 8'hFC: m3 = 1; 32'd12345: m3 = 2; default: m3 = 0; endcase
    $display("FA %0d %0d %0d %0d", m0, m1, m2, m3);
    e = 8'hF6;
    case (e) inside 8'hFC: m0 = 1; default: m0 = 0; endcase
    case (e) inside 8'hFC: m1 = 1; 8'h55: m1 = 2; default: m1 = 0; endcase
    case (e) inside 8'hFC: m2 = 1; 16'sh7777: m2 = 2; default: m2 = 0; endcase
    case (e) inside 8'hFC: m3 = 1; 32'd12345: m3 = 2; default: m3 = 0; endcase
    $display("F6 %0d %0d %0d %0d", m0, m1, m2, m3);
    e = 8'h06;
    case (e) inside 8'hFC: m0 = 1; default: m0 = 0; endcase
    case (e) inside 8'hFC: m1 = 1; 8'h55: m1 = 2; default: m1 = 0; endcase
    case (e) inside 8'hFC: m2 = 1; 16'sh7777: m2 = 2; default: m2 = 0; endcase
    case (e) inside 8'hFC: m3 = 1; 32'd12345: m3 = 2; default: m3 = 0; endcase
    $display("06 %0d %0d %0d %0d", m0, m1, m2, m3);
    e = 8'h0A;
    case (e) inside 8'hFC: m0 = 1; default: m0 = 0; endcase
    case (e) inside 8'hFC: m1 = 1; 8'h55: m1 = 2; default: m1 = 0; endcase
    case (e) inside 8'hFC: m2 = 1; 16'sh7777: m2 = 2; default: m2 = 0; endcase
    case (e) inside 8'hFC: m3 = 1; 32'd12345: m3 = 2; default: m3 = 0; endcase
    $display("0A %0d %0d %0d %0d", m0, m1, m2, m3);
    e = 8'h1E;
    case (e) inside 8'hFC: m0 = 1; default: m0 = 0; endcase
    case (e) inside 8'hFC: m1 = 1; 8'h55: m1 = 2; default: m1 = 0; endcase
    case (e) inside 8'hFC: m2 = 1; 16'sh7777: m2 = 2; default: m2 = 0; endcase
    case (e) inside 8'hFC: m3 = 1; 32'd12345: m3 = 2; default: m3 = 0; endcase
    $display("1E %0d %0d %0d %0d", m0, m1, m2, m3);
    e = 8'hEE;
    case (e) inside 8'hFC: m0 = 1; default: m0 = 0; endcase
    case (e) inside 8'hFC: m1 = 1; 8'h55: m1 = 2; default: m1 = 0; endcase
    case (e) inside 8'hFC: m2 = 1; 16'sh7777: m2 = 2; default: m2 = 0; endcase
    case (e) inside 8'hFC: m3 = 1; 32'd12345: m3 = 2; default: m3 = 0; endcase
    $display("EE %0d %0d %0d %0d", m0, m1, m2, m3);
    e = 8'hE6;
    case (e) inside 8'hFC: m0 = 1; default: m0 = 0; endcase
    case (e) inside 8'hFC: m1 = 1; 8'h55: m1 = 2; default: m1 = 0; endcase
    case (e) inside 8'hFC: m2 = 1; 16'sh7777: m2 = 2; default: m2 = 0; endcase
    case (e) inside 8'hFC: m3 = 1; 32'd12345: m3 = 2; default: m3 = 0; endcase
    $display("E6 %0d %0d %0d %0d", m0, m1, m2, m3);
    e = 8'hFC;
    case (e) inside 8'hFC: m0 = 1; default: m0 = 0; endcase
    case (e) inside 8'hFC: m1 = 1; 8'h55: m1 = 2; default: m1 = 0; endcase
    case (e) inside 8'hFC: m2 = 1; 16'sh7777: m2 = 2; default: m2 = 0; endcase
    case (e) inside 8'hFC: m3 = 1; 32'd12345: m3 = 2; default: m3 = 0; endcase
    $display("FC %0d %0d %0d %0d", m0, m1, m2, m3);
    e = 8'h0C;
    case (e) inside 8'hFC: m0 = 1; default: m0 = 0; endcase
    case (e) inside 8'hFC: m1 = 1; 8'h55: m1 = 2; default: m1 = 0; endcase
    case (e) inside 8'hFC: m2 = 1; 16'sh7777: m2 = 2; default: m2 = 0; endcase
    case (e) inside 8'hFC: m3 = 1; 32'd12345: m3 = 2; default: m3 = 0; endcase
    $display("0C %0d %0d %0d %0d", m0, m1, m2, m3);
    #10 $finish;
  end
endmodule

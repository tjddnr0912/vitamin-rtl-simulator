module top;
  logic signed [3:0] e; int m0, m1, m2, m3;
  initial begin
    e = 4'b0000;
    case (e) inside [8'hFC:8'hFE]: m0 = 1; default: m0 = 0; endcase
    case (e) inside [8'hFC:8'hFE]: m1 = 1; 8'h55: m1 = 2; default: m1 = 0; endcase
    case (e) inside [8'hFC:8'hFE]: m2 = 1; 16'sh7777: m2 = 2; default: m2 = 0; endcase
    case (e) inside [8'hFC:8'hFE]: m3 = 1; 32'd12345: m3 = 2; default: m3 = 0; endcase
    $display("4'b0000 %0d %0d %0d %0d", m0, m1, m2, m3);
    e = 4'b0001;
    case (e) inside [8'hFC:8'hFE]: m0 = 1; default: m0 = 0; endcase
    case (e) inside [8'hFC:8'hFE]: m1 = 1; 8'h55: m1 = 2; default: m1 = 0; endcase
    case (e) inside [8'hFC:8'hFE]: m2 = 1; 16'sh7777: m2 = 2; default: m2 = 0; endcase
    case (e) inside [8'hFC:8'hFE]: m3 = 1; 32'd12345: m3 = 2; default: m3 = 0; endcase
    $display("4'b0001 %0d %0d %0d %0d", m0, m1, m2, m3);
    e = 4'b0010;
    case (e) inside [8'hFC:8'hFE]: m0 = 1; default: m0 = 0; endcase
    case (e) inside [8'hFC:8'hFE]: m1 = 1; 8'h55: m1 = 2; default: m1 = 0; endcase
    case (e) inside [8'hFC:8'hFE]: m2 = 1; 16'sh7777: m2 = 2; default: m2 = 0; endcase
    case (e) inside [8'hFC:8'hFE]: m3 = 1; 32'd12345: m3 = 2; default: m3 = 0; endcase
    $display("4'b0010 %0d %0d %0d %0d", m0, m1, m2, m3);
    e = 4'b0011;
    case (e) inside [8'hFC:8'hFE]: m0 = 1; default: m0 = 0; endcase
    case (e) inside [8'hFC:8'hFE]: m1 = 1; 8'h55: m1 = 2; default: m1 = 0; endcase
    case (e) inside [8'hFC:8'hFE]: m2 = 1; 16'sh7777: m2 = 2; default: m2 = 0; endcase
    case (e) inside [8'hFC:8'hFE]: m3 = 1; 32'd12345: m3 = 2; default: m3 = 0; endcase
    $display("4'b0011 %0d %0d %0d %0d", m0, m1, m2, m3);
    e = 4'b0100;
    case (e) inside [8'hFC:8'hFE]: m0 = 1; default: m0 = 0; endcase
    case (e) inside [8'hFC:8'hFE]: m1 = 1; 8'h55: m1 = 2; default: m1 = 0; endcase
    case (e) inside [8'hFC:8'hFE]: m2 = 1; 16'sh7777: m2 = 2; default: m2 = 0; endcase
    case (e) inside [8'hFC:8'hFE]: m3 = 1; 32'd12345: m3 = 2; default: m3 = 0; endcase
    $display("4'b0100 %0d %0d %0d %0d", m0, m1, m2, m3);
    e = 4'b0101;
    case (e) inside [8'hFC:8'hFE]: m0 = 1; default: m0 = 0; endcase
    case (e) inside [8'hFC:8'hFE]: m1 = 1; 8'h55: m1 = 2; default: m1 = 0; endcase
    case (e) inside [8'hFC:8'hFE]: m2 = 1; 16'sh7777: m2 = 2; default: m2 = 0; endcase
    case (e) inside [8'hFC:8'hFE]: m3 = 1; 32'd12345: m3 = 2; default: m3 = 0; endcase
    $display("4'b0101 %0d %0d %0d %0d", m0, m1, m2, m3);
    e = 4'b0110;
    case (e) inside [8'hFC:8'hFE]: m0 = 1; default: m0 = 0; endcase
    case (e) inside [8'hFC:8'hFE]: m1 = 1; 8'h55: m1 = 2; default: m1 = 0; endcase
    case (e) inside [8'hFC:8'hFE]: m2 = 1; 16'sh7777: m2 = 2; default: m2 = 0; endcase
    case (e) inside [8'hFC:8'hFE]: m3 = 1; 32'd12345: m3 = 2; default: m3 = 0; endcase
    $display("4'b0110 %0d %0d %0d %0d", m0, m1, m2, m3);
    e = 4'b0111;
    case (e) inside [8'hFC:8'hFE]: m0 = 1; default: m0 = 0; endcase
    case (e) inside [8'hFC:8'hFE]: m1 = 1; 8'h55: m1 = 2; default: m1 = 0; endcase
    case (e) inside [8'hFC:8'hFE]: m2 = 1; 16'sh7777: m2 = 2; default: m2 = 0; endcase
    case (e) inside [8'hFC:8'hFE]: m3 = 1; 32'd12345: m3 = 2; default: m3 = 0; endcase
    $display("4'b0111 %0d %0d %0d %0d", m0, m1, m2, m3);
    e = 4'b1000;
    case (e) inside [8'hFC:8'hFE]: m0 = 1; default: m0 = 0; endcase
    case (e) inside [8'hFC:8'hFE]: m1 = 1; 8'h55: m1 = 2; default: m1 = 0; endcase
    case (e) inside [8'hFC:8'hFE]: m2 = 1; 16'sh7777: m2 = 2; default: m2 = 0; endcase
    case (e) inside [8'hFC:8'hFE]: m3 = 1; 32'd12345: m3 = 2; default: m3 = 0; endcase
    $display("4'b1000 %0d %0d %0d %0d", m0, m1, m2, m3);
    e = 4'b1001;
    case (e) inside [8'hFC:8'hFE]: m0 = 1; default: m0 = 0; endcase
    case (e) inside [8'hFC:8'hFE]: m1 = 1; 8'h55: m1 = 2; default: m1 = 0; endcase
    case (e) inside [8'hFC:8'hFE]: m2 = 1; 16'sh7777: m2 = 2; default: m2 = 0; endcase
    case (e) inside [8'hFC:8'hFE]: m3 = 1; 32'd12345: m3 = 2; default: m3 = 0; endcase
    $display("4'b1001 %0d %0d %0d %0d", m0, m1, m2, m3);
    e = 4'b1010;
    case (e) inside [8'hFC:8'hFE]: m0 = 1; default: m0 = 0; endcase
    case (e) inside [8'hFC:8'hFE]: m1 = 1; 8'h55: m1 = 2; default: m1 = 0; endcase
    case (e) inside [8'hFC:8'hFE]: m2 = 1; 16'sh7777: m2 = 2; default: m2 = 0; endcase
    case (e) inside [8'hFC:8'hFE]: m3 = 1; 32'd12345: m3 = 2; default: m3 = 0; endcase
    $display("4'b1010 %0d %0d %0d %0d", m0, m1, m2, m3);
    e = 4'b1011;
    case (e) inside [8'hFC:8'hFE]: m0 = 1; default: m0 = 0; endcase
    case (e) inside [8'hFC:8'hFE]: m1 = 1; 8'h55: m1 = 2; default: m1 = 0; endcase
    case (e) inside [8'hFC:8'hFE]: m2 = 1; 16'sh7777: m2 = 2; default: m2 = 0; endcase
    case (e) inside [8'hFC:8'hFE]: m3 = 1; 32'd12345: m3 = 2; default: m3 = 0; endcase
    $display("4'b1011 %0d %0d %0d %0d", m0, m1, m2, m3);
    e = 4'b1100;
    case (e) inside [8'hFC:8'hFE]: m0 = 1; default: m0 = 0; endcase
    case (e) inside [8'hFC:8'hFE]: m1 = 1; 8'h55: m1 = 2; default: m1 = 0; endcase
    case (e) inside [8'hFC:8'hFE]: m2 = 1; 16'sh7777: m2 = 2; default: m2 = 0; endcase
    case (e) inside [8'hFC:8'hFE]: m3 = 1; 32'd12345: m3 = 2; default: m3 = 0; endcase
    $display("4'b1100 %0d %0d %0d %0d", m0, m1, m2, m3);
    e = 4'b1101;
    case (e) inside [8'hFC:8'hFE]: m0 = 1; default: m0 = 0; endcase
    case (e) inside [8'hFC:8'hFE]: m1 = 1; 8'h55: m1 = 2; default: m1 = 0; endcase
    case (e) inside [8'hFC:8'hFE]: m2 = 1; 16'sh7777: m2 = 2; default: m2 = 0; endcase
    case (e) inside [8'hFC:8'hFE]: m3 = 1; 32'd12345: m3 = 2; default: m3 = 0; endcase
    $display("4'b1101 %0d %0d %0d %0d", m0, m1, m2, m3);
    e = 4'b1110;
    case (e) inside [8'hFC:8'hFE]: m0 = 1; default: m0 = 0; endcase
    case (e) inside [8'hFC:8'hFE]: m1 = 1; 8'h55: m1 = 2; default: m1 = 0; endcase
    case (e) inside [8'hFC:8'hFE]: m2 = 1; 16'sh7777: m2 = 2; default: m2 = 0; endcase
    case (e) inside [8'hFC:8'hFE]: m3 = 1; 32'd12345: m3 = 2; default: m3 = 0; endcase
    $display("4'b1110 %0d %0d %0d %0d", m0, m1, m2, m3);
    e = 4'b1111;
    case (e) inside [8'hFC:8'hFE]: m0 = 1; default: m0 = 0; endcase
    case (e) inside [8'hFC:8'hFE]: m1 = 1; 8'h55: m1 = 2; default: m1 = 0; endcase
    case (e) inside [8'hFC:8'hFE]: m2 = 1; 16'sh7777: m2 = 2; default: m2 = 0; endcase
    case (e) inside [8'hFC:8'hFE]: m3 = 1; 32'd12345: m3 = 2; default: m3 = 0; endcase
    $display("4'b1111 %0d %0d %0d %0d", m0, m1, m2, m3);
    #10 $finish;
  end
endmodule

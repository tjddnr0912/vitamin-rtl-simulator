module top;
  logic [7:0] a, b; logic [3:0] n; logic [8:0] w; string s; int m, cnt;
  function automatic logic [3:0] g(input int k); cnt = cnt + 1; return k[3:0]; endfunction
  initial begin
    a = 8'h80; b = 8'h80; n = 4'b1010; s = "hi"; cnt = 0;
    case (a + b) 9'h100: m = 1; default: m = 0; endcase $display("p1 %0d", m);
    case ((1:a + b:2)) 9'h100: m = 1; default: m = 0; endcase $display("p2 %0d", m);
    casez (n) 4'b1?1?: m = 1; default: m = 0; endcase $display("p3 %0d", m);
    casex (n) 4'bx0x0: m = 1; default: m = 0; endcase $display("p4 %0d", m);
    case (n) '1: m = 1; 4'b1010: m = 2; default: m = 0; endcase $display("p5 %0d", m);
    case (s) "hi": m = 1; default: m = 0; endcase $display("p6 %0d", m);
    case (g(3)) 4'd1: m = 1; 4'd2: m = 2; 4'd3: m = 3; default: m = 0; endcase $display("p7 %0d cnt=%0d", m, cnt);
    w = 9'h1FF;
    case (w[3:0] - 4'd1) 5'h1E: m = 1; 4'hE: m = 2; default: m = 0; endcase $display("p9 %0d", m);
    #1 $finish;
  end
endmodule

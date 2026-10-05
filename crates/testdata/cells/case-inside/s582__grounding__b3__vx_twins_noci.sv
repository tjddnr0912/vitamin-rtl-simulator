module top;
  logic [7:0] e; logic signed [7:0] es; logic [3:0] e4; int m;
  initial begin
    e = 8'h80; es = 8'sh80; e4 = 4'b1010;
    case (e) 8'sh80: m = 1; 16'sh7777: m = 2; default: m = 0; endcase        $display("A pc  u8 vs 8'sh80 +16'sh7777   m=%0d", m);
    case (es) 8'sh80: m = 1; 32'd12345: m = 2; default: m = 0; endcase        $display("B pc  s8 vs 8'sh80 +32'd12345   m=%0d", m);
    casez (e4) 4'sb1?10: m = 1; 8'h55: m = 2; default: m = 0; endcase         $display("C cz  u4 vs 4'sb1?10 +8'h55     m=%0d", m);
    m = (e4 ==? 4'sb1?10);                                                   $display("C weq u4 ==? 4'sb1?10           m=%0d", m);
    e = 8'h0E;
    m = (e ==? 4'sb1?10);                                                    $display("D weq u8=0E ==? 4'sb1?10        m=%0d", m);
    e = 8'hFE;
    m = (e ==? 4'sb1?10);                                                    $display("E weq u8=FE ==? 4'sb1?10        m=%0d", m);
    #10 $finish;
  end
endmodule

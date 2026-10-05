module top;
  int cnt = 0; int m;
  function automatic logic [3:0] f(input int n); cnt++; $display("f(%0d) call %0d", n, cnt); return n[3:0]; endfunction
  initial begin
    case (f(3)) inside 4'd1: m = 1; [4'd5:4'd6]: m = 2; 4'b1?00: m = 3; 4'd3: m = 4; default: m = 0; endcase
    $display("A m=%0d cnt=%0d", m, cnt);
    cnt = 0;
    case (f(9)) inside 4'd1, 4'd2: m = 1; default: m = 0; endcase
    $display("B m=%0d cnt=%0d", m, cnt);
    cnt = 0;
    case (f(7)) inside default: m = 5; endcase
    $display("C m=%0d cnt=%0d", m, cnt);
    #10 $finish;
  end
endmodule

module top;
  int m;
  function automatic logic [3:0] f(input int n); $display("f(%0d)", n); return n[3:0]; endfunction
  initial begin
    case (f(3)) 4'd1: m = 1; 4'd5: m = 2; 4'd3: m = 4; default: m = 0; endcase
    $display("A m=%0d", m);
    case (f(7)) default: m = 5; endcase
    $display("C m=%0d", m);
    #10 $finish;
  end
endmodule

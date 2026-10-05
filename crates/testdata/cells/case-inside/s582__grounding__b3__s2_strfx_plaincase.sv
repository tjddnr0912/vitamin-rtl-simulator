module top;
  int m;
  function automatic string sf(input int n); $display("sf(%0d)", n); return (n == 1) ? "b" : "z"; endfunction
  initial begin
    case (sf(1)) "a": m = 1; "c": m = 2; "b": m = 3; default: m = 0; endcase
    $display("m=%0d", m);
    #10 $finish;
  end
endmodule

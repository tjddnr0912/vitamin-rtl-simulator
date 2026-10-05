module top;
  int cnt, lo, m; bit [7:0] v8; bit [15:0] v16;
  function automatic int f(); cnt = cnt + 1; return 5; endfunction
  function automatic int g(); lo = lo + 10; return 12; endfunction
  initial begin
    cnt = 4;
    case (f()) inside cnt: m = 1; default: m = 0; endcase
    $display("A item reads var E's call wrote: m=%0d cnt=%0d", m, cnt);
    lo = 0;
    case (g()) inside [lo:20]: m = 1; [0:9]: m = 2; default: m = 0; endcase
    $display("B range bound E's call wrote: m=%0d lo=%0d", m, lo);
    v8 = 8'h62;
    case (v8) inside "b": m = 1; "a": m = 2; default: m = 0; endcase
    $display("C v8=62 in {\"b\"} m=%0d", m);
    v16 = 16'h6162;
    case (v16) inside "ab": m = 1; "b": m = 2; default: m = 0; endcase
    $display("D v16=6162 in {\"ab\"} m=%0d", m);
    v16 = 16'h0062;
    case (v16) inside "ab": m = 1; "b": m = 2; default: m = 0; endcase
    $display("E v16=0062 in {\"b\"} m=%0d", m);
    #1 $finish;
  end
endmodule

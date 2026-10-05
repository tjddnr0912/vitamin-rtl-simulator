module top;
  string s; int m;
  initial begin
    s = "am"; case (s) inside ["aa":"az"]: m = 1; "b": m = 2; default: m = 0; endcase $display("am m=%0d", m);
    s = "b";  case (s) inside ["aa":"az"]: m = 1; "b": m = 2; default: m = 0; endcase $display("b m=%0d", m);
    s = "ba"; case (s) inside ["aa":"az"]: m = 1; "b": m = 2; default: m = 0; endcase $display("ba m=%0d", m);
    s = "a";  case (s) inside ["aa":"az"]: m = 1; "b": m = 2; default: m = 0; endcase $display("a m=%0d", m);
    #10 $finish;
  end
endmodule
